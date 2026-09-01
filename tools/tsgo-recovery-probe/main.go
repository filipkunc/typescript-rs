// This probe is copied into a pinned TypeScript-Go checkout before it is run. Keeping the command
// inside that module is required by Go's internal-package import rule.
package main

import (
	"context"
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"

	"github.com/microsoft/typescript-go/internal/ast"
	"github.com/microsoft/typescript-go/internal/binder"
	"github.com/microsoft/typescript-go/internal/bundled"
	"github.com/microsoft/typescript-go/internal/compiler"
	"github.com/microsoft/typescript-go/internal/core"
	"github.com/microsoft/typescript-go/internal/parser"
	"github.com/microsoft/typescript-go/internal/tsoptions"
	"github.com/microsoft/typescript-go/internal/tspath"
	"github.com/microsoft/typescript-go/internal/vfs/vfstest"
)

const schemaVersion = 1

type manifest struct {
	SchemaVersion  int            `json:"schemaVersion"`
	Implementation string         `json:"implementation"`
	Revision       string         `json:"revision"`
	Cases          []caseManifest `json:"cases"`
}

type caseManifest struct {
	ID               string         `json:"id"`
	Source           string         `json:"source"`
	TopLevel         []topLevelNode `json:"topLevel"`
	DeclarationNames []string       `json:"declarationNames"`
	BindingNames     []string       `json:"bindingNames"`
	Diagnostics      []diagnostic   `json:"diagnostics"`
	RecoveryNodes    []recoveryNode `json:"recoveryNodes"`
}

type topLevelNode struct {
	Kind  string   `json:"kind"`
	Start int      `json:"start"`
	End   int      `json:"end"`
	Names []string `json:"names"`
}

type diagnostic struct {
	Code    int32  `json:"code"`
	Start   int    `json:"start"`
	End     int    `json:"end"`
	Message string `json:"message"`
}

type recoveryNode struct {
	Kind        string   `json:"kind"`
	Start       int      `json:"start"`
	End         int      `json:"end"`
	Missing     bool     `json:"missing"`
	DirectError bool     `json:"directError"`
	ParentPath  []string `json:"parentPath"`
}

type inspectionNode struct {
	Kind      string           `json:"kind"`
	Start     int              `json:"start"`
	End       int              `json:"end"`
	Recovered bool             `json:"recovered"`
	Children  []inspectionNode `json:"children"`
}

type inspectionRange struct {
	Start int `json:"start"`
	End   int `json:"end"`
}

type inspectionDiagnostic struct {
	Code    string            `json:"code"`
	Phase   string            `json:"phase"`
	Message string            `json:"message"`
	Labels  []inspectionRange `json:"labels"`
}

type inspectionRecoverySite struct {
	Kind            string   `json:"kind"`
	Start           int      `json:"start"`
	End             int      `json:"end"`
	DiagnosticIndex *int     `json:"diagnosticIndex,omitempty"`
	ParentPath      []string `json:"parentPath"`
}

type semanticSummary struct {
	BindingNames   []string `json:"bindingNames"`
	ReferenceCount *int     `json:"referenceCount,omitempty"`
}

type liveInspection struct {
	Implementation    string                   `json:"implementation"`
	Revision          string                   `json:"revision"`
	Mode              string                   `json:"mode"`
	Status            string                   `json:"status"`
	StatementCount    int                      `json:"statementCount"`
	DiagnosticCount   int                      `json:"diagnosticCount"`
	RecoverySiteCount int                      `json:"recoverySiteCount"`
	Diagnostics       []inspectionDiagnostic   `json:"diagnostics"`
	Tree              inspectionNode           `json:"tree"`
	RecoverySites     []inspectionRecoverySite `json:"recoverySites"`
	DeclarationNames  []string                 `json:"declarationNames"`
	Semantic          semanticSummary          `json:"semantic"`
}

type inspectionRequest struct {
	FileName string `json:"fileName"`
	Source   string `json:"source"`
}

func main() {
	casesDirectory := flag.String("cases", "", "directory containing recovery .ts cases")
	revision := flag.String("revision", "", "pinned TypeScript-Go revision")
	output := flag.String("output", "", "output JSON path; stdout when omitted")
	serve := flag.String("serve", "", "serve live recovery inspection on this address")
	flag.Parse()

	if *revision == "" || (*casesDirectory == "" && *serve == "") {
		fmt.Fprintln(os.Stderr, "--revision and either --cases or --serve are required")
		os.Exit(2)
	}
	if *serve != "" {
		serveInspections(*serve, *revision)
		return
	}

	paths, err := filepath.Glob(filepath.Join(*casesDirectory, "*.ts"))
	check(err)
	sort.Strings(paths)
	result := manifest{
		SchemaVersion:  schemaVersion,
		Implementation: "typescript-go",
		Revision:       *revision,
		Cases:          make([]caseManifest, 0, len(paths)),
	}
	for _, path := range paths {
		result.Cases = append(result.Cases, inspect(path))
	}

	encoded, err := json.MarshalIndent(result, "", "  ")
	check(err)
	encoded = append(encoded, '\n')
	if *output == "" {
		_, err = os.Stdout.Write(encoded)
	} else {
		err = os.WriteFile(*output, encoded, 0o644)
	}
	check(err)
}

func serveInspections(address string, revision string) {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /api/typescript-go/health", func(writer http.ResponseWriter, _ *http.Request) {
		writeJSON(writer, http.StatusOK, map[string]string{
			"implementation": "typescript-go",
			"revision":       revision,
		})
	})
	mux.HandleFunc("POST /api/typescript-go/inspect", func(writer http.ResponseWriter, request *http.Request) {
		request.Body = http.MaxBytesReader(writer, request.Body, 2<<20)
		decoder := json.NewDecoder(request.Body)
		decoder.DisallowUnknownFields()
		var input inspectionRequest
		if err := decoder.Decode(&input); err != nil {
			writeJSON(writer, http.StatusBadRequest, map[string]string{"error": err.Error()})
			return
		}
		if err := ensureJSONBodyConsumed(decoder); err != nil {
			writeJSON(writer, http.StatusBadRequest, map[string]string{"error": err.Error()})
			return
		}
		if input.FileName == "" {
			writeJSON(writer, http.StatusBadRequest, map[string]string{
				"error": "fileName is required",
			})
			return
		}
		writeJSON(writer, http.StatusOK, inspectLive(request.Context(), input.FileName, input.Source, revision))
	})

	server := &http.Server{
		Addr:              address,
		Handler:           mux,
		ReadHeaderTimeout: 5 * time.Second,
		ReadTimeout:       10 * time.Second,
		WriteTimeout:      10 * time.Second,
		IdleTimeout:       30 * time.Second,
	}
	fmt.Printf("TypeScript-Go recovery reference listening on http://%s\n", address)
	if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
		check(err)
	}
}

func ensureJSONBodyConsumed(decoder *json.Decoder) error {
	var extra any
	err := decoder.Decode(&extra)
	if err == io.EOF {
		return nil
	}
	if err == nil {
		return fmt.Errorf("request body must contain one JSON object")
	}
	return err
}

func writeJSON(writer http.ResponseWriter, status int, value any) {
	writer.Header().Set("Content-Type", "application/json")
	writer.WriteHeader(status)
	if err := json.NewEncoder(writer).Encode(value); err != nil {
		fmt.Fprintln(os.Stderr, err)
	}
}

func inspectLive(ctx context.Context, fileName string, source string, revision string) liveInspection {
	fileName = filepath.ToSlash(filepath.Join("/playground", filepath.Base(fileName)))
	file := parser.ParseSourceFile(ast.SourceFileParseOptions{
		FileName: fileName,
		Path:     tspath.Path(fileName),
	}, source, core.EnsureScriptKindFromFileName(fileName))
	parseDiagnosticCount := len(file.Diagnostics())
	binder.BindSourceFile(file)

	diagnostics := make([]inspectionDiagnostic, 0, len(file.Diagnostics()))
	for index, item := range file.Diagnostics() {
		phase := "parse"
		if index >= parseDiagnosticCount {
			phase = "bind"
		}
		diagnostics = append(diagnostics, inspectionDiagnostic{
			Code:    fmt.Sprintf("TS%d", item.Code()),
			Phase:   phase,
			Message: item.String(),
			Labels: []inspectionRange{{
				Start: item.Pos(),
				End:   item.End(),
			}},
		})
	}
	for _, item := range checkLive(ctx, fileName, source) {
		diagnostics = append(diagnostics, inspectionDiagnostic{
			Code:    fmt.Sprintf("TS%d", item.Code()),
			Phase:   "check",
			Message: item.String(),
			Labels: []inspectionRange{{
				Start: item.Pos(),
				End:   item.End(),
			}},
		})
	}

	recoveryNodes := make([]recoveryNode, 0)
	walkRecoveryNodes(file.AsNode(), nil, &recoveryNodes)
	recoverySites := make([]inspectionRecoverySite, 0, len(recoveryNodes))
	for _, node := range recoveryNodes {
		recoverySites = append(recoverySites, inspectionRecoverySite{
			Kind:            node.Kind,
			Start:           node.Start,
			End:             node.End,
			DiagnosticIndex: matchingDiagnostic(node.Start, node.End, diagnostics, source),
			ParentPath:      node.ParentPath,
		})
	}

	declarationSet := make(map[string]struct{})
	for _, statement := range file.Statements.Nodes {
		for _, name := range declarationNamesWithin(statement, file.AsNode().Locals()) {
			declarationSet[name] = struct{}{}
		}
	}

	status := "clean"
	if len(diagnostics) != 0 {
		status = "recovered"
	}
	return liveInspection{
		Implementation:    "typescript-go",
		Revision:          revision,
		Mode:              "reference",
		Status:            status,
		StatementCount:    len(file.Statements.Nodes),
		DiagnosticCount:   len(diagnostics),
		RecoverySiteCount: len(recoverySites),
		Diagnostics:       diagnostics,
		Tree:              buildInspectionTree(file.AsNode()),
		RecoverySites:     recoverySites,
		DeclarationNames:  sortedSet(declarationSet),
		Semantic: semanticSummary{
			BindingNames: sortedKeys(file.AsNode().Locals()),
		},
	}
}

func checkLive(ctx context.Context, fileName string, source string) []*ast.Diagnostic {
	fs := vfstest.FromMap(map[string]string{fileName: source}, true)
	fs = bundled.WrapFS(fs)
	options := &core.CompilerOptions{
		Module:            core.ModuleKindESNext,
		NoEmit:            core.TSTrue,
		NoErrorTruncation: core.TSTrue,
		SkipLibCheck:      core.TSTrue,
		Strict:            core.TSTrue,
		Target:            core.ScriptTargetESNext,
	}
	program := compiler.NewProgram(compiler.ProgramOptions{
		Config: &tsoptions.ParsedCommandLine{
			ParsedConfig: &tsoptions.ParsedOptions{
				CompilerOptions: options,
				FileNames:       []string{fileName},
			},
		},
		Host: compiler.NewCompilerHost("/playground", fs, bundled.LibPath(), nil, nil, nil),
	})
	programFile := program.GetSourceFile(fileName)
	if programFile == nil {
		return nil
	}
	typeChecker, release := program.GetTypeCheckerForFile(ctx, programFile)
	defer release()
	return typeChecker.GetDiagnostics(ctx, programFile)
}

func buildInspectionTree(node *ast.Node) inspectionNode {
	children := make([]inspectionNode, 0)
	node.ForEachChild(func(child *ast.Node) bool {
		children = append(children, buildInspectionTree(child))
		return false
	})
	return inspectionNode{
		Kind:      strings.TrimPrefix(node.KindString(), "Kind"),
		Start:     node.Pos(),
		End:       node.End(),
		Recovered: ast.NodeIsMissing(node) || node.Flags&ast.NodeFlagsThisNodeHasError != 0,
		Children:  children,
	}
}

func matchingDiagnostic(start int, end int, diagnostics []inspectionDiagnostic, source string) *int {
	for index, diagnostic := range diagnostics {
		for _, label := range diagnostic.Labels {
			if (start == end && label.Start <= start && start <= label.End) ||
				(label.Start <= end && start <= label.End) ||
				(start == end && end <= label.Start && label.Start <= len(source) &&
					strings.TrimSpace(source[end:label.Start]) == "") {
				matched := index
				return &matched
			}
		}
	}
	return nil
}

func inspect(path string) caseManifest {
	sourceBytes, err := os.ReadFile(path)
	check(err)
	source := string(sourceBytes)
	abs, err := filepath.Abs(path)
	check(err)
	file := parser.ParseSourceFile(ast.SourceFileParseOptions{
		FileName: abs,
		Path:     tspath.Path(abs),
	}, source, core.ScriptKindTS)
	binder.BindSourceFile(file)

	bindings := sortedKeys(file.AsNode().Locals())
	topLevel := make([]topLevelNode, 0, len(file.Statements.Nodes))
	declarationSet := make(map[string]struct{})
	for _, statement := range file.Statements.Nodes {
		names := declarationNamesWithin(statement, file.AsNode().Locals())
		for _, name := range names {
			declarationSet[name] = struct{}{}
		}
		topLevel = append(topLevel, topLevelNode{
			Kind:  normalizeTopLevelKind(statement.Kind),
			Start: statement.Pos(),
			End:   statement.End(),
			Names: names,
		})
	}

	diagnostics := make([]diagnostic, 0, len(file.Diagnostics()))
	for _, item := range file.Diagnostics() {
		diagnostics = append(diagnostics, diagnostic{
			Code:    item.Code(),
			Start:   item.Pos(),
			End:     item.End(),
			Message: item.String(),
		})
	}

	recoveryNodes := make([]recoveryNode, 0)
	walkRecoveryNodes(file.AsNode(), nil, &recoveryNodes)
	return caseManifest{
		ID:               strings.TrimSuffix(filepath.Base(path), filepath.Ext(path)),
		Source:           source,
		TopLevel:         topLevel,
		DeclarationNames: sortedSet(declarationSet),
		BindingNames:     bindings,
		Diagnostics:      diagnostics,
		RecoveryNodes:    recoveryNodes,
	}
}

func declarationNamesWithin(statement *ast.Node, locals ast.SymbolTable) []string {
	names := make([]string, 0)
	for name, symbol := range locals {
		for _, declaration := range symbol.Declarations {
			if statement.Pos() <= declaration.Pos() && declaration.End() <= statement.End() {
				names = append(names, name)
				break
			}
		}
	}
	sort.Strings(names)
	return names
}

func walkRecoveryNodes(node *ast.Node, path []string, output *[]recoveryNode) {
	if node == nil {
		return
	}
	kind := strings.TrimPrefix(node.KindString(), "Kind")
	missing := ast.NodeIsMissing(node)
	directError := node.Flags&ast.NodeFlagsThisNodeHasError != 0
	if missing || directError {
		*output = append(*output, recoveryNode{
			Kind:        kind,
			Start:       node.Pos(),
			End:         node.End(),
			Missing:     missing,
			DirectError: directError,
			ParentPath:  append([]string(nil), path...),
		})
	}
	nextPath := append(append([]string(nil), path...), kind)
	node.ForEachChild(func(child *ast.Node) bool {
		walkRecoveryNodes(child, nextPath, output)
		return false
	})
}

func normalizeTopLevelKind(kind ast.Kind) string {
	switch kind {
	case ast.KindVariableStatement:
		return "variable"
	case ast.KindExpressionStatement:
		return "expression"
	case ast.KindFunctionDeclaration:
		return "function"
	case ast.KindInterfaceDeclaration:
		return "interface"
	case ast.KindClassDeclaration:
		return "class"
	case ast.KindTypeAliasDeclaration:
		return "typeAlias"
	default:
		return strings.TrimPrefix(kind.String(), "Kind")
	}
}

func sortedKeys(table ast.SymbolTable) []string {
	keys := make([]string, 0, len(table))
	for key := range table {
		if !strings.HasPrefix(key, ast.InternalSymbolNamePrefix) {
			keys = append(keys, key)
		}
	}
	sort.Strings(keys)
	return keys
}

func sortedSet(values map[string]struct{}) []string {
	keys := make([]string, 0, len(values))
	for key := range values {
		keys = append(keys, key)
	}
	sort.Strings(keys)
	return keys
}

func check(err error) {
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
