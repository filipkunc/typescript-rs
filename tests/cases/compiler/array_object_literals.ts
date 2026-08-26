// @reference: bootstrap/array-object-literals
type User = { id: number; name?: string };
type Batch = { users: User[] };

const users: User[] = [{ id: 1 }, { id: 2, name: "Ada" }];
const direct: { id: number; active: boolean }[] = [
    { id: 3, active: true },
    { id: 4, active: false },
];
const batch: Batch = { users: [{ id: 5, name: "Grace" }] };
