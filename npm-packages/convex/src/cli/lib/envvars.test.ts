import {afterEach, beforeEach, expect, test, vi} from "vitest";
import {writeConvexUrlToEnvFile} from "./envvars.js";
import {Context} from "../../bundler/context.js";

vi.mock("./utils/utils.js", () => ({
	loadPackageJson: vi.fn().mockResolvedValue({name: "test-project"}),
	ENV_VAR_FILE_PATH: ".env.local",
}));

const mockContext = {
	fs: {
		exists: vi.fn() as any,
		readUtf8File: vi.fn() as any,
		writeUtf8File: vi.fn() as any,
	},
	crash: vi.fn() as any,
} as unknown as Context;

const originalProcessEnv = process.env;

beforeEach(() => {
	vi.clearAllMocks();
	process.env = {...originalProcessEnv};
});

afterEach(() => {
	process.env = originalProcessEnv;
});

test("writeConvexUrlToEnvFile process.env behavior", async () => {
	// Test core functionality: skip file creation when env var exists with correct value
	process.env.CONVEX_URL = "https://test.convex.cloud";
	(mockContext.fs.exists as any).mockReturnValue(false);

	let result = await writeConvexUrlToEnvFile(mockContext, "https://test.convex.cloud");
	expect(result).toBeNull(); // Should skip file creation
	expect(mockContext.fs.writeUtf8File).not.toHaveBeenCalled();

	// Test different value - should create file
	vi.clearAllMocks();
	process.env.CONVEX_URL = "https://different.convex.cloud";

	result = await writeConvexUrlToEnvFile(mockContext, "https://test.convex.cloud");
	expect(result).not.toBeNull(); // Should create file
	expect(mockContext.fs.writeUtf8File).toHaveBeenCalled();

	// Test missing env var - should create file
	vi.clearAllMocks();
	delete process.env.CONVEX_URL;

	result = await writeConvexUrlToEnvFile(mockContext, "https://test.convex.cloud");
	expect(result).not.toBeNull(); // Should create file
	expect(mockContext.fs.writeUtf8File).toHaveBeenCalled();

	// Empty string should trigger file creation
	vi.clearAllMocks();
	process.env.CONVEX_URL = "";
	result = await writeConvexUrlToEnvFile(mockContext, "https://test.convex.cloud");
	expect(result).not.toBeNull();

	// Whitespace should trigger file creation
	vi.clearAllMocks();
	process.env.CONVEX_URL = "  ";
	result = await writeConvexUrlToEnvFile(mockContext, "https://test.convex.cloud");
	expect(result).not.toBeNull();
});
