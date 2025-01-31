import { functionIdentifierValue } from "lib/functions/generateFileTree";
import {
  functionsForSelectedNents,
  selectNentOption,
} from "features/logs/components/LogToolbar";

describe("selectNentOption", () => {
  const nents = ["_App", "nent1", "nent2"];
  const functions = [
    functionIdentifierValue("func1"),
    functionIdentifierValue("func2", "nent1", "id1"),
    functionIdentifierValue("func3", "nent1", "id1"),
    functionIdentifierValue("func4", "nent2", "id2"),
  ];

  let setSelectedFunctions: jest.Mock;
  let setSelectedNents: jest.Mock;

  beforeEach(() => {
    setSelectedFunctions = jest.fn();
    setSelectedNents = jest.fn();
  });

  const testCases = [
    {
      name: "removing all nents removes all functions",
      nents,
      functions,
      newNents: [],
      expectedSelectedNents: [],
      expectedSelectedFunctions: [],
    },
    {
      name: "removing two nents removes functions related to those nents",
      nents,
      functions,
      newNents: ["_App"],
      expectedSelectedNents: ["_App"],
      expectedSelectedFunctions: [functions[0]],
    },
    {
      name: "adding two nents adds functions related to those nents",
      nents: [],
      functions: [],
      newNents: ["_App", "nent2"],
      expectedSelectedNents: ["_App", "nent2"],
      expectedSelectedFunctions: [functions[0], functions[3]],
    },
    {
      name: "adding a nent does not add functions related to other nents",
      nents: ["_App"],
      functions: [],
      newNents: ["_App", "nent2"],
      expectedSelectedNents: ["_App", "nent2"],
      expectedSelectedFunctions: [functions[3]],
    },
  ];

  testCases.forEach(
    ({
      name,
      nents: selectedNents,
      functions: selectedFunctions,
      newNents,
      expectedSelectedNents,
      expectedSelectedFunctions,
    }) => {
      test(name, () => {
        const updateSelectedNents = selectNentOption({
          selectedNents,
          functions,
          selectedFunctions,
          setSelectedFunctions,
          setSelectedNents,
        });

        updateSelectedNents(newNents);

        expect(setSelectedNents).toHaveBeenCalledWith(expectedSelectedNents);
        expect(setSelectedFunctions).toHaveBeenCalledWith(
          expectedSelectedFunctions,
        );
      });
    },
  );
});

describe("functionsForSelectedNents", () => {
  const functions = [
    functionIdentifierValue("func1"),
    functionIdentifierValue("func2", "nent1", "id1"),
    functionIdentifierValue("func3", "nent1", "id1"),
    functionIdentifierValue("func4", "nent2", "id2"),
  ];

  const testCases = [
    {
      name: "returns no functions when no nents are selected",
      nents: [],
      expectedFunctions: [],
    },
    {
      name: "returns only functions related to selected nents",
      nents: ["_App", "nent1"],
      expectedFunctions: [functions[0], functions[1], functions[2]],
    },
    {
      name: "returns only functions related to selected nents",
      nents: ["_App", "nent2"],
      expectedFunctions: [functions[0], functions[3]],
    },
    {
      name: "returns all functions when all nents are selected",
      nents: ["_App", "nent1", "nent2"],
      expectedFunctions: functions,
    },
    {
      name: "returns app functions when only _App is selected",
      nents: ["_App"],
      expectedFunctions: [functions[0]],
    },
    {
      name: "returns nent functions when only _App is selected",
      nents: ["nent1", "nent2"],
      expectedFunctions: functions.slice(1),
    },
  ];

  testCases.forEach(({ name, nents, expectedFunctions }) => {
    test(name, () => {
      const result = functionsForSelectedNents(nents, functions);

      expect(result).toEqual(expectedFunctions);
    });
  });
});
