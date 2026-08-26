// @reference: bootstrap/array-object-element-properties
type User = { id: number; name: string };

const missingName: User[] = [{ id: 1 }];
const wrongName: User[] = [{ id: 2, name: false }];
