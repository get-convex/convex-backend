import { Id } from "system-udfs/convex/_generated/dataModel";
import {
  Module,
  UdfType,
  Visibility,
  CronJobLog,
  CronJobWithLastRun,
} from "system-udfs/convex/_system/frontend/common";

export interface FileTreeItem {
  name: string;
  componentPath: string | null;
  componentId: Id<"_components"> | null;
  // This is usually a file path, but may be something like
  // "GET /listMessages" for HTTP actions
  identifier: string;
  // The unique identifier for this item is computed by `itemIdentifier`.
}

export interface ModuleFunction extends FileTreeItem {
  type: "function";
  lineno?: number;
  displayName: string;
  udfType: UdfType;
  visibility: Visibility;
  file: { name: string; identifier: string };
  args: string;
}

export interface File extends FileTreeItem {
  type: "file";
  functions: ModuleFunction[];
  componentPath: string | null;
}

export interface Folder extends FileTreeItem {
  type: "folder";
  children: FileOrFolder[];
}

export type FileOrFolder = File | Folder;

export type CronJobsContextType = {
  cronsModule: Module | undefined;
  cronJobs: CronJobWithLastRun[] | undefined;
  loading: boolean;
  cronJobRuns: CronJobLog[] | undefined;
};
