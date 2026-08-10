import { createAuthStore, registerAuthStoreGetter } from "@shared";
import { authApi } from "@/api";

export const useAuthStore = createAuthStore({
  id: "auth-city3d",
  appKind: "user",
  allowedRoles: ["city3d"],
  authApi,
});

registerAuthStoreGetter(() => useAuthStore());