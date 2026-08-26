// @reference: bootstrap/nested-structural-diagnostics
type Root = {
    profile: { name: string; contact?: { email: string } };
    users: { id: number; tags: string[] }[];
    matrix: number[][];
};

const invalid: Root = {
    profile: {},
    users: [
        { id: "one", tags: ["ok", false] },
        {},
    ],
    matrix: [[1, "two"]],
};
