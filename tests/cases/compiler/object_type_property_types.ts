// @reference: bootstrap/object-type-property-types
type User = { id: number; name: string };

const wrongId: User = { id: "1", name: "Ada" };
const wrongName: User = { id: 2, name: false };
