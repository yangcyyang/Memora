import {
  createHashHistory,
  createRootRoute,
  createRoute,
  createRouter,
} from "@tanstack/react-router";

import App from "./App";
import { WelcomeView } from "@/features/onboarding/WelcomeView";
import { DashboardView } from "@/features/dashboard/DashboardView";
import { CreateWizard } from "@/features/create/CreateWizard";
import { ChatView } from "@/features/chat/ChatView";
import { SettingsView } from "@/features/settings/SettingsView";
import { ProfileView } from "@/features/profile/ProfileView";
import { DataImport } from "@/features/import";

// ── Root ────────────────────────────────────────────────────────────
const rootRoute = createRootRoute({ component: App });

// ── Routes ──────────────────────────────────────────────────────────
export const welcomeRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/welcome",
  component: WelcomeView,
});

export const dashboardRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  component: DashboardView,
});

export const createRoute_ = createRoute({
  getParentRoute: () => rootRoute,
  path: "/create",
  component: CreateWizard,
});

export const chatRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/chat/$personaId",
  component: ChatView,
});

export const profileRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/profile/$personaId",
  component: ProfileView,
});

export const settingsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/settings",
  component: SettingsView,
});

export const importRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/import",
  component: DataImport,
});

// ── Route Tree ──────────────────────────────────────────────────────
const routeTree = rootRoute.addChildren([
  welcomeRoute,
  dashboardRoute,
  createRoute_,
  chatRoute,
  profileRoute,
  settingsRoute,
  importRoute,
]);

// ── Router ──────────────────────────────────────────────────────────
export const router = createRouter({
  routeTree,
  history: createHashHistory(),
});

// Type safety
declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
