// This probe is copied into a pinned TypeScript-Go checkout before it is run. Keeping the command
// inside that module is required by Go's internal-package import rule.
package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"

	"github.com/microsoft/typescript-go/internal/ast"
	"github.com/microsoft/typescript-go/internal/binder"
	"github.com/microsoft/typescript-go/internal/core"
	"github.com/microsoft/typescript-go/internal/parser"
	"github.com/microsoft/typescript-go/internal/tspath"
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

func main() {
	casesDirectory := flag.String("cases", "", "directory containing recovery .ts cases")
	revision := flag.String("revision", "", "pinned TypeScript-Go revision")
	output := flag.String("output", "", "output JSON path; stdout when omitted")
	flag.Parse()

	if *casesDirectory == "" || *revision == "" {
		fmt.Fprintln(os.Stderr, "--cases and --revision are required")
		os.Exit(2)
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
