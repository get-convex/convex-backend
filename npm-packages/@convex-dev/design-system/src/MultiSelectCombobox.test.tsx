import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MultiSelectCombobox, MultiSelectValue } from "./MultiSelectCombobox";

function CustomOption({
  label,
  inButton,
}: {
  label: string;
  inButton: boolean;
}) {
  void inButton;
  return <span data-testid={`custom-${label}`}>{label.toUpperCase()}</span>;
}

describe("MultiSelectCombobox", () => {
  beforeEach(jest.clearAllMocks);

  const defaultProps = {
    options: ["Apple", "Banana", "Cherry", "Date", "Elderberry"],
    unit: "item",
    unitPlural: "items",
    label: "Select Items",
  };

  const setup = (
    props: Partial<typeof defaultProps> & {
      selectedOptions: MultiSelectValue;
      setSelectedOptions: (value: MultiSelectValue) => void;
    },
  ) => render(<MultiSelectCombobox {...defaultProps} {...props} />);

  describe("Basic rendering", () => {
    it("renders with label and button", () => {
      const setSelectedOptions = jest.fn();
      setup({ selectedOptions: [], setSelectedOptions });

      expect(screen.getByText("Select Items")).toBeInTheDocument();
      expect(screen.getByRole("button")).toBeInTheDocument();
    });

    it("renders with hidden label when labelHidden is true", () => {
      const setSelectedOptions = jest.fn();
      const { container } = render(
        <MultiSelectCombobox
          {...defaultProps}
          selectedOptions={[]}
          setSelectedOptions={setSelectedOptions}
          labelHidden
        />,
      );

      const label = container.querySelector("label");
      expect(label).toHaveClass("hidden");
    });

    it("displays correct count when no items selected", () => {
      const setSelectedOptions = jest.fn();
      setup({ selectedOptions: [], setSelectedOptions });

      expect(screen.getByText("0 items")).toBeInTheDocument();
    });

    it("displays correct count when one item selected", () => {
      const setSelectedOptions = jest.fn();
      setup({ selectedOptions: ["Apple"], setSelectedOptions });

      expect(screen.getByText("1 item")).toBeInTheDocument();
    });

    it("displays correct count when multiple items selected", () => {
      const setSelectedOptions = jest.fn();
      setup({ selectedOptions: ["Apple", "Banana"], setSelectedOptions });

      expect(screen.getByText("2 items")).toBeInTheDocument();
    });

    it("displays 'All items' when selectedOptions is 'all'", () => {
      const setSelectedOptions = jest.fn();
      setup({ selectedOptions: "all", setSelectedOptions });

      expect(screen.getByText("All items")).toBeInTheDocument();
    });
  });

  describe("Opening and closing dropdown", () => {
    it("opens dropdown when button is clicked", async () => {
      const setSelectedOptions = jest.fn();
      setup({ selectedOptions: [], setSelectedOptions });

      const user = userEvent.setup();
      const button = screen.getByRole("button");

      await user.click(button);

      await waitFor(() => {
        expect(screen.getByText("Select all")).toBeInTheDocument();
      });
    });

    it("closes dropdown when button is clicked again", async () => {
      const setSelectedOptions = jest.fn();
      setup({ selectedOptions: [], setSelectedOptions });

      const user = userEvent.setup();
      const button = screen.getByRole("button");

      await user.click(button);
      await waitFor(() => {
        expect(screen.getByText("Select all")).toBeInTheDocument();
      });

      await user.click(button);
      await waitFor(() => {
        expect(screen.queryByText("Select all")).not.toBeInTheDocument();
      });
    });

    it("displays all options when dropdown is open", async () => {
      const setSelectedOptions = jest.fn();
      setup({ selectedOptions: [], setSelectedOptions });

      const user = userEvent.setup();
      await user.click(screen.getByRole("button"));

      await waitFor(() => {
        expect(screen.getByText("Apple")).toBeInTheDocument();
        expect(screen.getByText("Banana")).toBeInTheDocument();
        expect(screen.getByText("Cherry")).toBeInTheDocument();
        expect(screen.getByText("Date")).toBeInTheDocument();
        expect(screen.getByText("Elderberry")).toBeInTheDocument();
      });
    });
  });

  describe("Selecting and deselecting options", () => {
    it("selects an option when clicked", async () => {
      const setSelectedOptions = jest.fn();
      setup({ selectedOptions: [], setSelectedOptions });

      const user = userEvent.setup();
      await user.click(screen.getByRole("button"));

      await waitFor(() => {
        expect(screen.getByText("Apple")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Apple"));

      expect(setSelectedOptions).toHaveBeenCalledWith(["Apple"]);
    });

    it("deselects an option when clicked again", async () => {
      const setSelectedOptions = jest.fn();
      setup({ selectedOptions: ["Apple"], setSelectedOptions });

      const user = userEvent.setup();
      await user.click(screen.getByRole("button"));

      await waitFor(() => {
        expect(screen.getByText("Apple")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Apple"));

      expect(setSelectedOptions).toHaveBeenCalledWith([]);
    });

    it("selects multiple options", async () => {
      const setSelectedOptions = jest.fn();
      setup({ selectedOptions: ["Apple"], setSelectedOptions });

      const user = userEvent.setup();
      await user.click(screen.getByRole("button"));

      await waitFor(() => {
        expect(screen.getByText("Banana")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Banana"));

      expect(setSelectedOptions).toHaveBeenCalledWith(["Apple", "Banana"]);
    });

    it("converts to 'all' state when all options are selected", async () => {
      const setSelectedOptions = jest.fn();
      setup({
        selectedOptions: ["Apple", "Banana", "Cherry", "Date"],
        setSelectedOptions,
      });

      const user = userEvent.setup();
      await user.click(screen.getByRole("button"));

      await waitFor(() => {
        expect(screen.getByText("Elderberry")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Elderberry"));

      expect(setSelectedOptions).toHaveBeenCalledWith("all");
    });
  });

  describe("Select all / Deselect all functionality", () => {
    it("shows 'Select all' button when not all items are selected", async () => {
      const setSelectedOptions = jest.fn();
      setup({ selectedOptions: [], setSelectedOptions });

      const user = userEvent.setup();
      await user.click(screen.getByRole("button"));

      await waitFor(() => {
        expect(screen.getByText("Select all")).toBeInTheDocument();
      });
    });

    it("shows 'Deselect all' button when all items are selected", async () => {
      const setSelectedOptions = jest.fn();
      setup({ selectedOptions: "all", setSelectedOptions });

      const user = userEvent.setup();
      await user.click(screen.getByRole("button"));

      await waitFor(() => {
        expect(screen.getByText("Deselect all")).toBeInTheDocument();
      });
    });

    it("selects all options when 'Select all' is clicked", async () => {
      const setSelectedOptions = jest.fn();
      setup({ selectedOptions: [], setSelectedOptions });

      const user = userEvent.setup();
      await user.click(screen.getByRole("button"));

      await waitFor(() => {
        expect(screen.getByText("Select all")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Select all"));

      expect(setSelectedOptions).toHaveBeenCalledWith("all");
    });

    it("deselects all options when 'Deselect all' is clicked", async () => {
      const setSelectedOptions = jest.fn();
      setup({ selectedOptions: "all", setSelectedOptions });

      const user = userEvent.setup();
      await user.click(screen.getByRole("button"));

      await waitFor(() => {
        expect(screen.getByText("Deselect all")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Deselect all"));

      expect(setSelectedOptions).toHaveBeenCalledWith([]);
    });
  });

  describe("Search functionality", () => {
    it("displays search input by default", async () => {
      const setSelectedOptions = jest.fn();
      setup({ selectedOptions: [], setSelectedOptions });

      const user = userEvent.setup();
      await user.click(screen.getByRole("button"));

      await waitFor(() => {
        expect(
          screen.getByPlaceholderText("Search items..."),
        ).toBeInTheDocument();
      });
    });

    it("does not display search input when disableSearch is true", async () => {
      const setSelectedOptions = jest.fn();
      render(
        <MultiSelectCombobox
          {...defaultProps}
          selectedOptions={[]}
          setSelectedOptions={setSelectedOptions}
          disableSearch
        />,
      );

      const user = userEvent.setup();
      await user.click(screen.getByRole("button"));

      await waitFor(() => {
        expect(screen.getByText("Select all")).toBeInTheDocument();
      });

      expect(
        screen.queryByPlaceholderText("Search items..."),
      ).not.toBeInTheDocument();
    });

    it("filters options based on search query", async () => {
      const setSelectedOptions = jest.fn();
      setup({ selectedOptions: [], setSelectedOptions });

      const user = userEvent.setup();
      await user.click(screen.getByRole("button"));

      await waitFor(() => {
        expect(
          screen.getByPlaceholderText("Search items..."),
        ).toBeInTheDocument();
      });

      const searchInput = screen.getByPlaceholderText("Search items...");
      await user.type(searchInput, "ban");

      await waitFor(() => {
        expect(screen.getByText("Banana")).toBeInTheDocument();
        expect(screen.queryByText("Apple")).not.toBeInTheDocument();
        expect(screen.queryByText("Cherry")).not.toBeInTheDocument();
      });
    });

    it("shows all options when search query is cleared", async () => {
      const setSelectedOptions = jest.fn();
      setup({ selectedOptions: [], setSelectedOptions });

      const user = userEvent.setup();
      await user.click(screen.getByRole("button"));

      await waitFor(() => {
        expect(
          screen.getByPlaceholderText("Search items..."),
        ).toBeInTheDocument();
      });

      const searchInput = screen.getByPlaceholderText("Search items...");
      await user.type(searchInput, "ban");

      await waitFor(() => {
        expect(screen.getByText("Banana")).toBeInTheDocument();
        expect(screen.queryByText("Apple")).not.toBeInTheDocument();
      });

      await user.clear(searchInput);

      await waitFor(() => {
        expect(screen.getByText("Apple")).toBeInTheDocument();
        expect(screen.getByText("Banana")).toBeInTheDocument();
        expect(screen.getByText("Cherry")).toBeInTheDocument();
      });
    });
  });

  describe("'Only' button functionality", () => {
    it("displays 'only' button on hover for each option", async () => {
      const setSelectedOptions = jest.fn();
      setup({ selectedOptions: [], setSelectedOptions });

      const user = userEvent.setup();
      await user.click(screen.getByRole("button"));

      await waitFor(() => {
        expect(screen.getByText("Apple")).toBeInTheDocument();
      });

      const appleOption = screen.getByText("Apple").closest("li");
      expect(appleOption).toBeInTheDocument();

      const onlyButtons = screen.getAllByText("only");
      expect(onlyButtons.length).toBeGreaterThan(0);
    });

    it("selects only the clicked option when 'only' is clicked", async () => {
      const setSelectedOptions = jest.fn();
      setup({ selectedOptions: ["Apple", "Banana"], setSelectedOptions });

      const user = userEvent.setup();
      await user.click(screen.getByRole("button"));

      await waitFor(() => {
        expect(screen.getByText("Cherry")).toBeInTheDocument();
      });

      const onlyButtons = screen.getAllByText("only");
      const cherryOnlyButton = onlyButtons[2];

      await user.click(cherryOnlyButton);

      expect(setSelectedOptions).toHaveBeenCalledWith(["Cherry"]);
    });
  });

  describe("Max displayed options", () => {
    it("shows message when more than 100 options are available", async () => {
      const manyOptions = Array.from(
        { length: 150 },
        (_, i) => `Option ${i + 1}`,
      );
      const setSelectedOptions = jest.fn();

      render(
        <MultiSelectCombobox
          {...defaultProps}
          options={manyOptions}
          selectedOptions={[]}
          setSelectedOptions={setSelectedOptions}
        />,
      );

      const user = userEvent.setup();
      await user.click(screen.getByRole("button"));

      await waitFor(() => {
        expect(
          screen.getByText(
            /Too many items to display, use the searchbar to filter items/,
          ),
        ).toBeInTheDocument();
      });
    });

    it("displays only first 100 options when more are available", async () => {
      const manyOptions = Array.from(
        { length: 150 },
        (_, i) => `Option ${i + 1}`,
      );
      const setSelectedOptions = jest.fn();

      render(
        <MultiSelectCombobox
          {...defaultProps}
          options={manyOptions}
          selectedOptions={[]}
          setSelectedOptions={setSelectedOptions}
        />,
      );

      const user = userEvent.setup();
      await user.click(screen.getByRole("button"));

      await waitFor(() => {
        expect(screen.getByText("Option 1")).toBeInTheDocument();
        expect(screen.getByText("Option 100")).toBeInTheDocument();
        expect(screen.queryByText("Option 101")).not.toBeInTheDocument();
      });
    });

    it("filters down options when searching with many options", async () => {
      const manyOptions = Array.from(
        { length: 150 },
        (_, i) => `Option ${i + 1}`,
      );
      const setSelectedOptions = jest.fn();

      render(
        <MultiSelectCombobox
          {...defaultProps}
          options={manyOptions}
          selectedOptions={[]}
          setSelectedOptions={setSelectedOptions}
        />,
      );

      const user = userEvent.setup();
      await user.click(screen.getByRole("button"));

      await waitFor(() => {
        expect(
          screen.getByPlaceholderText("Search items..."),
        ).toBeInTheDocument();
      });

      const searchInput = screen.getByPlaceholderText("Search items...");
      await user.type(searchInput, "Option 1 ");

      await waitFor(() => {
        expect(screen.getByText("Option 1")).toBeInTheDocument();
        expect(screen.queryByText("Option 2")).not.toBeInTheDocument();
      });
    });
  });

  describe("Custom Option component", () => {
    it("renders custom Option component when provided", async () => {
      const setSelectedOptions = jest.fn();

      render(
        <MultiSelectCombobox
          {...defaultProps}
          selectedOptions={[]}
          setSelectedOptions={setSelectedOptions}
          Option={CustomOption}
        />,
      );

      const user = userEvent.setup();
      await user.click(screen.getByRole("button"));

      await waitFor(() => {
        expect(screen.getByTestId("custom-Apple")).toBeInTheDocument();
        expect(screen.getByText("APPLE")).toBeInTheDocument();
      });
    });
  });

  describe("processFilterOption", () => {
    it("uses processFilterOption to filter options", async () => {
      const setSelectedOptions = jest.fn();
      const processFilterOption = (option: string) => option.toLowerCase();

      render(
        <MultiSelectCombobox
          {...defaultProps}
          selectedOptions={[]}
          setSelectedOptions={setSelectedOptions}
          processFilterOption={processFilterOption}
        />,
      );

      const user = userEvent.setup();
      await user.click(screen.getByRole("button"));

      await waitFor(() => {
        expect(
          screen.getByPlaceholderText("Search items..."),
        ).toBeInTheDocument();
      });

      const searchInput = screen.getByPlaceholderText("Search items...");
      await user.type(searchInput, "APPLE");

      await waitFor(() => {
        expect(screen.getByText("Apple")).toBeInTheDocument();
      });
    });
  });

  describe("Accessibility", () => {
    it("has proper ARIA attributes", () => {
      const setSelectedOptions = jest.fn();
      setup({ selectedOptions: [], setSelectedOptions });

      const button = screen.getByRole("button");
      expect(button).toBeInTheDocument();
    });

    it("sets tabindex to 0 on the button", async () => {
      const setSelectedOptions = jest.fn();
      setup({ selectedOptions: [], setSelectedOptions });

      await waitFor(() => {
        const button = screen.getByRole("button");
        expect(button).toHaveAttribute("tabindex", "0");
      });
    });
  });

  describe("Edge cases", () => {
    it("handles empty options array", async () => {
      const setSelectedOptions = jest.fn();
      render(
        <MultiSelectCombobox
          {...defaultProps}
          options={[]}
          selectedOptions={[]}
          setSelectedOptions={setSelectedOptions}
        />,
      );

      const user = userEvent.setup();
      await user.click(screen.getByRole("button"));

      await waitFor(() => {
        expect(screen.getByText("Select all")).toBeInTheDocument();
      });
    });

    it("filters out _other from count", () => {
      const setSelectedOptions = jest.fn();
      setup({ selectedOptions: ["Apple", "_other"], setSelectedOptions });

      expect(screen.getByText("1 item")).toBeInTheDocument();
    });

    it("handles selectedOptions with _other correctly", () => {
      const setSelectedOptions = jest.fn();
      setup({
        selectedOptions: ["Apple", "Banana", "_other"],
        setSelectedOptions,
      });

      expect(screen.getByText("2 items")).toBeInTheDocument();
    });
  });
});
