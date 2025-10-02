import { render, screen, fireEvent, act } from "@testing-library/react";
import {
  useDeleteProfileEmail,
  useResendProfileEmailVerification,
  useUpdatePrimaryProfileEmail,
} from "api/profile";
import userEvent from "@testing-library/user-event";
import { MemberEmailResponse } from "generatedApi";
import { EmailListItem } from "./EmailListItem";

// Mock the useDeleteProfileEmail hook
jest.mock("api/profile", () => ({
  useDeleteProfileEmail: jest.fn(),
  useUpdatePrimaryProfileEmail: jest.fn(),
  useResendProfileEmailVerification: jest.fn(),
  useIdentities: jest.fn(),
}));

const mockDeleteProfileEmail = useDeleteProfileEmail as jest.Mock;
const mockUpdatePrimaryProfileEmail = useUpdatePrimaryProfileEmail as jest.Mock;
const mockResendProfileEmailVerification =
  useResendProfileEmailVerification as jest.Mock;

describe("EmailListItem", () => {
  const email: MemberEmailResponse = {
    email: "example@example.com",
    isPrimary: false,
    isVerified: false,
    id: 0,
    creationTime: 0,
  };

  beforeEach(() => {
    mockDeleteProfileEmail.mockClear();
  });

  it("renders the email", () => {
    render(<EmailListItem email={email} />);
    expect(screen.getByText(email.email)).toBeInTheDocument();
  });

  it('shows "Primary" badge for primary email', () => {
    render(
      <EmailListItem
        email={{
          ...email,
          isPrimary: true,
        }}
      />,
    );
    expect(screen.getByText("Primary")).toBeInTheDocument();
  });

  it('shows "Verified" badge for verified email', () => {
    render(<EmailListItem email={{ ...email, isVerified: true }} />);
    expect(screen.getByText("Verified")).toBeInTheDocument();
  });

  it('shows "Unverified" badge for unverified email', () => {
    render(<EmailListItem email={email} />);
    expect(screen.getByText("Unverified")).toBeInTheDocument();
  });

  it("disables delete button for primary email", async () => {
    render(
      <EmailListItem
        email={{
          ...email,
          isPrimary: true,
        }}
      />,
    );

    await act(async () => {
      fireEvent.click(screen.getByTestId("open-menu"));
    });
    expect(screen.getByText("Delete")).toBeDisabled();
  });

  it("enables delete button for non-primary email", async () => {
    render(<EmailListItem email={email} />);

    await act(async () => {
      fireEvent.click(screen.getByTestId("open-menu"));
    });
    expect(screen.getByText("Delete")).toBeEnabled();
  });

  it("shows confirmation dialog when delete button is clicked", async () => {
    render(<EmailListItem email={email} />);
    await act(async () => {
      fireEvent.click(screen.getByTestId("open-menu"));
    });
    await act(async () => {
      fireEvent.click(screen.getByText("Delete"));
    });
    expect(screen.getByText("Delete Email")).toBeInTheDocument();
  });

  it("calls deleteEmail function when confirm is clicked", async () => {
    const deleteEmail = jest.fn();
    mockDeleteProfileEmail.mockReturnValue(deleteEmail);
    render(<EmailListItem email={email} />);
    await act(async () => {
      fireEvent.click(screen.getByTestId("open-menu"));
    });
    fireEvent.click(screen.getByText("Delete"));
    await userEvent.type(screen.getByLabelText("validation"), email.email);
    await act(async () => {
      fireEvent.click(screen.getByText("Delete"));
    });
    expect(deleteEmail).toHaveBeenCalledWith({ email: email.email });
  });

  it("shows error message if deleteEmail fails", async () => {
    const errorMessage = "Failed to delete email";
    mockDeleteProfileEmail.mockReturnValue(() => {
      throw new Error(errorMessage);
    });
    render(<EmailListItem email={email} />);
    await act(async () => {
      fireEvent.click(screen.getByTestId("open-menu"));
    });
    fireEvent.click(screen.getByText("Delete"));
    await userEvent.type(screen.getByLabelText("validation"), email.email);
    await act(async () => {
      fireEvent.click(screen.getByText("Delete"));
    });

    expect(await screen.findByText(errorMessage)).toBeInTheDocument();
  });

  it("disables 'Set as primary' button for unverified email", async () => {
    render(<EmailListItem email={email} />);
    await act(async () => {
      fireEvent.click(screen.getByTestId("open-menu"));
    });
    expect(screen.getByText("Set as primary")).toBeDisabled();
  });

  it("enables 'Set as primary' button for verified email", async () => {
    render(<EmailListItem email={{ ...email, isVerified: true }} />);
    await act(async () => {
      fireEvent.click(screen.getByTestId("open-menu"));
    });
    expect(screen.getByText("Set as primary")).toBeEnabled();
  });

  it("calls updatePrimaryEmail function when 'Set as primary' is clicked", async () => {
    const updatePrimaryEmail = jest.fn();
    mockUpdatePrimaryProfileEmail.mockReturnValue(updatePrimaryEmail);
    render(<EmailListItem email={{ ...email, isVerified: true }} />);
    await act(async () => {
      fireEvent.click(screen.getByTestId("open-menu"));
    });
    fireEvent.click(screen.getByText("Set as primary"));
    expect(updatePrimaryEmail).toHaveBeenCalledWith({ email: email.email });
  });

  it("shows 'Resend verification email' option for unverified email", async () => {
    render(<EmailListItem email={email} />);
    await act(async () => {
      fireEvent.click(screen.getByTestId("open-menu"));
    });
    expect(screen.getByText("Resend verification email")).toBeInTheDocument();
  });

  it("does not show 'Resend verification email' option for verified email", async () => {
    render(<EmailListItem email={{ ...email, isVerified: true }} />);
    await act(async () => {
      fireEvent.click(screen.getByTestId("open-menu"));
    });
    expect(
      screen.queryByText("Resend verification email"),
    ).not.toBeInTheDocument();
  });

  it("calls resendEmailVerification function when 'Resend verification email' is clicked", async () => {
    const resendEmailVerification = jest.fn();
    mockResendProfileEmailVerification.mockReturnValue(resendEmailVerification);
    render(<EmailListItem email={email} />);
    await act(async () => {
      fireEvent.click(screen.getByTestId("open-menu"));
    });
    fireEvent.click(screen.getByText("Resend verification email"));
    expect(resendEmailVerification).toHaveBeenCalledWith({
      email: email.email,
    });
  });
});
