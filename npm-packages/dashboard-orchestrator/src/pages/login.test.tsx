import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import LoginPage from "./login";

const mockReplace = jest.fn();
const mockUseRouter = jest.fn();
const mockSignInEmail = jest.fn();

jest.mock("next/router", () => ({
  useRouter: () => mockUseRouter(),
}));

jest.mock("../lib/auth-client", () => ({
  authClient: {
    signIn: {
      email: mockSignInEmail,
      social: jest.fn(),
    },
    signUp: {
      email: jest.fn(),
    },
  },
}));

describe("LoginPage", () => {
  beforeEach(() => {
    mockReplace.mockReset();
    mockSignInEmail.mockReset();
    mockSignInEmail.mockResolvedValue({});
    mockUseRouter.mockReturnValue({
      query: {},
      replace: mockReplace,
    });
  });

  test("renders email and password fields together for browser autofill", () => {
    render(<LoginPage />);

    expect(screen.getByLabelText("Email")).toHaveAttribute(
      "autocomplete",
      "username",
    );
    expect(screen.getByLabelText("Password")).toHaveAttribute(
      "autocomplete",
      "current-password",
    );
  });

  test("keeps the password field mounted when the email changes", async () => {
    const user = userEvent.setup();
    render(<LoginPage />);

    await user.type(screen.getByLabelText("Email"), "first@example.com");
    await user.type(screen.getByLabelText("Password"), "password123");
    await user.clear(screen.getByLabelText("Email"));
    await user.type(screen.getByLabelText("Email"), "second@example.com");

    expect(screen.getByLabelText("Password")).toHaveValue("password123");
  });

  test("submits values already present in the DOM from browser autofill", async () => {
    const user = userEvent.setup();
    render(<LoginPage />);

    setNativeInputValue(screen.getByLabelText("Email"), "saved@example.com");
    setNativeInputValue(screen.getByLabelText("Password"), "password123");

    await user.click(screen.getByRole("button", { name: "Sign in" }));

    await waitFor(() =>
      expect(mockSignInEmail).toHaveBeenCalledWith({
        email: "saved@example.com",
        password: "password123",
      }),
    );
  });
});

function setNativeInputValue(element: HTMLElement, value: string) {
  const valueSetter = Object.getOwnPropertyDescriptor(
    HTMLInputElement.prototype,
    "value",
  )?.set;
  valueSetter?.call(element, value);
}
