export const hasAllPermissions = (requiredPermissions: string[] = [], userPermissions: string[] = []) =>
  requiredPermissions.every((permission) => userPermissions?.includes(permission));
