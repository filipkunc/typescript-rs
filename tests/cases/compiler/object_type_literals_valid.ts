// @reference: bootstrap/object-type-literals-valid
type User = { id: number; name: string };
type ReorderedUser = { name: string; id: number };

const direct: { id: number; name: string } = { id: 1, name: "Ada" };
const alias: User = { id: 2, name: "Grace" };
const reordered: ReorderedUser = { id: 3, name: "Lin" };
