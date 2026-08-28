// @reference bootstrap/property-only-interfaces/properties-valid

type Status = "active" | "disabled";

interface Address {
    city: string;
    coordinates: number[];
}

export interface User {
    id: number;
    name?: string;
    status: Status;
    address: Address;
}

export default interface Directory {
    owner: User;
    members: Array<User>;
}

const directory: Directory = {
    owner: {
        id: 1,
        status: "active",
        address: { city: "Vienna", coordinates: [48.2, 16.37] },
    },
    members: [
        {
            id: 2,
            name: "Ada",
            status: "disabled",
            address: { city: "London", coordinates: [51.5, -0.12] },
        },
    ],
};
