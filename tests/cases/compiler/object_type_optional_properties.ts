// @reference: bootstrap/object-type-optional-properties
type User = { id: number; name?: string };

const omitted: User = { id: 1 };
const present: User = { id: 2, name: "Ada" };
const wrong: User = { id: 3, name: false };
