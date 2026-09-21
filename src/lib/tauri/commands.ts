import { invoke } from "@tauri-apps/api/core";
import type { AppMapping, Category, ContextAssessment, MyDay, PolicyDebug } from "@/types";

/** Typed wrappers around the Rust commands. Nothing else calls `invoke` directly. */

export const getMyDay = () => invoke<MyDay>("get_my_day");

export const listAppMappings = () => invoke<AppMapping[]>("list_app_mappings");

export const setAppCategory = (bundleId: string, category: Category) =>
  invoke<void>("set_app_category", { bundleId, category });

export const resetAppCategory = (bundleId: string) =>
  invoke<void>("reset_app_category", { bundleId });

export const getContextAssessment = () => invoke<ContextAssessment>("get_context_assessment");

export const getPolicyDebug = () => invoke<PolicyDebug>("get_policy_debug");
