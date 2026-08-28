// @reference bootstrap/property-only-interfaces/structural-errors

interface Profile {
    name: string;
    tags: string[];
}

interface User {
    id: number;
    profile: Profile;
}

const missingId: User = {
    profile: { name: "Ada", tags: [] },
};

const wrongNested: User = {
    id: 1,
    profile: { name: "Ada", tags: ["compiler", false] },
};

const excessNested: User = {
    id: 2,
    profile: { name: "Grace", tags: [], admin: true },
};
