// @reference: bootstrap/json-shaped-values-complete
type Status = "active" | "disabled";
type Member = { id: number; score: number | null };
type Group = { name: string; members: Member[] };
type Root = {
    id: number;
    status: Status;
    owner?: { name: string };
    tags: string[];
    groupPages: Group[][];
};

const value: Root = {
    id: -1,
    status: "active",
    tags: ["compiler", "json"],
    groupPages: [
        [
            {
                name: "maintainers",
                members: [
                    { id: 1, score: null },
                    { id: 2, score: -5 },
                ],
            },
        ],
        [],
    ],
};
