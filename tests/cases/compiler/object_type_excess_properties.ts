// @reference: bootstrap/object-type-excess-properties
type User = { id: number; name: string };
type Envelope = { user: User; groups: User[][] };

const topLevel: User = { id: 1, name: "Ada", admin: true };
const nested: Envelope = {
    user: { id: 2, name: "Grace", admin: true },
    groups: [[{ id: 3, name: "Lin", admin: true }]],
};
