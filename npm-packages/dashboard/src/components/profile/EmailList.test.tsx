import { render, screen, fireEvent, act } from "@testing-library/react";
import { MemberEmailResponse } from "generatedApi";
import { EmailList } from "./EmailList";

jest.mock("api/profile", () => ({
  useCreateProfileEmail: jest.fn(),
  useDeleteProfileEmail: jest.fn(),
  useUpdatePrimaryProfileEmail: jest.fn(),
  useResendProfileEmailVerification: jest.fn(),
}));

const mockEmails: MemberEmailResponse[] = [
  {
    id: 1,
    email: "email1@example.com",
    isPrimary: true,
    isVerified: true,
    creationTime: 0,
  },
  {
    id: 2,
    email: "email2@example.com",
    isPrimary: false,
    isVerified: false,
    creationTime: 0,
  },
];

describe("EmailList", () => {
  it("renders email list items", () => {
    render(<EmailList emails={mockEmails} />);

    expect(screen.getByText("email1@example.com")).toBeInTheDocument();
    expect(screen.getByText("email2@example.com")).toBeInTheDocument();
  });

  it('opens the add email modal when "Add email" button is clicked', async () => {
    render(<EmailList emails={mockEmails} />);

    await act(async () => {
      fireEvent.click(screen.getByText("Add email"));
    });

    expect(screen.getByTestId("email-create-form")).toBeInTheDocument();
  });

  it("closes the add email modal when the modal onClose is triggered", async () => {
    render(<EmailList emails={mockEmails} />);

    await act(async () => {
      fireEvent.click(screen.getByText("Add email"));
    });

    expect(screen.getByTestId("email-create-form")).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("close-panel-button"));

    expect(screen.queryByTestId("email-create-form")).not.toBeInTheDocument();
  });
});
