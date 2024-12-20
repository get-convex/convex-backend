export type LogLevel = "debug" | "info" | "warn" | "error";

export class Logger {
  level: LogLevel = "error";
  setLevel(level: LogLevel) {
    this.level = level;
  }
  debug(...args: any[]) {
    if (this.level === "debug") {
      console.log(`[DEBUG] ${new Date().toISOString()}`, ...args);
    }
  }
  info(...args: any[]) {
    if (["debug", "info"].includes(this.level)) {
      console.log(`[INFO] ${new Date().toISOString()}`, ...args);
    }
  }
  warn(...args: any[]) {
    if (["debug", "info", "warn"].includes(this.level)) {
      console.warn(...args);
    }
  }
  error(...args: any[]) {
    console.error(...args);
  }
}
