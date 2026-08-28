// @reference bootstrap/property-only-interfaces/callable-interactions

interface User {
    id: number;
    name: string;
}

function identity(user: User, enabled: boolean): User {
    return user;
}

function createUser(): User {
    return { id: 1, name: "Ada" };
}

function invalidUser(): User {
    return 1;
}

const created: User = createUser();
const selected: User = identity({ id: 2, name: "Grace" }, true);
identity({ id: 3, name: "Lin" }, "enabled");
