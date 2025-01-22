import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { useCreateProfileEmail } from "api/profile";
import { MemberEmailResponse } from "generatedApi";
import { EmailCreateForm } from "./EmailCreateForm";

jest.mock("api/profile", () => ({
  useCreateProfileEmail: jest.fn(),
}));

const mockUseCreateProfileEmail = useCreateProfileEmail as jest.Mock;

describe("EmailCreateForm", () => {
  const emails: MemberEmailResponse[] = [
    {
      email: "test@example.com",
      id: 0,
      isVerified: false,
      isPrimary: true,
      creationTime: 0,
    },
  ];
  const onCreate = jest.fn();

  beforeEach(() => {
    mockUseCreateProfileEmail.mockReturnValue(jest.fn());
  });

  afterEach(() => {
    jest.clearAllMocks();
  });

  it("renders the form with initial state", () => {
    render(<EmailCreateForm emails={emails} onCreate={onCreate} />);

    expect(screen.getByPlaceholderText("Email")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /save/i })).toBeDisabled();
  });

  it("validates email input and shows error for invalid email", async () => {
    render(<EmailCreateForm emails={emails} onCreate={onCreate} />);

    const emailInput = screen.getByPlaceholderText("Email");
    fireEvent.change(emailInput, { target: { value: "invalid-email" } });
    fireEvent.blur(emailInput);

    await waitFor(() => {
      expect(
        screen.getByText(/email must be a valid email/i),
      ).toBeInTheDocument();
    });
  });

  it("shows error for duplicate email", async () => {
    render(<EmailCreateForm emails={emails} onCreate={onCreate} />);

    const emailInput = screen.getByPlaceholderText("Email");
    fireEvent.change(emailInput, { target: { value: "test@example.com" } });
    fireEvent.blur(emailInput);

    await waitFor(() => {
      expect(
        screen.getByText(/this email is already associated with your account/i),
      ).toBeInTheDocument();
    });
  });

  it("submits the form with valid email", async () => {
    const createEmailMock = jest.fn().mockResolvedValue({});
    mockUseCreateProfileEmail.mockReturnValue(createEmailMock);

    render(<EmailCreateForm emails={emails} onCreate={onCreate} />);

    const emailInput = screen.getByPlaceholderText("Email");
    const submitButton = screen.getByRole("button", { name: /save/i });

    fireEvent.change(emailInput, { target: { value: "new@example.com" } });
    fireEvent.blur(emailInput);

    await waitFor(() => {
      expect(submitButton).not.toBeDisabled();
    });

    fireEvent.click(submitButton);

    await waitFor(() => {
      expect(createEmailMock).toHaveBeenCalledWith({
        email: "new@example.com",
      });
      expect(onCreate).toHaveBeenCalled();
    });
  });

  it("shows error message when submission fails", async () => {
    const createEmailMock = jest
      .fn()
      .mockRejectedValue(new Error("Submission failed"));
    mockUseCreateProfileEmail.mockReturnValue(createEmailMock);

    render(<EmailCreateForm emails={emails} onCreate={onCreate} />);

    const emailInput = screen.getByPlaceholderText("Email");
    const submitButton = screen.getByRole("button", { name: /save/i });

    fireEvent.change(emailInput, { target: { value: "new@example.com" } });
    fireEvent.blur(emailInput);

    await waitFor(() => {
      expect(submitButton).not.toBeDisabled();
    });

    fireEvent.click(submitButton);

    await waitFor(() => {
      expect(createEmailMock).toHaveBeenCalledWith({
        email: "new@example.com",
      });
      expect(screen.getByText("Submission failed")).toBeInTheDocument();
    });
  });
});
