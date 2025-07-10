import { Meta, StoryObj } from "@storybook/nextjs";
import { DeploymentResponse, ProjectDetails } from "generatedApi";
import { AggregatedFunctionMetrics } from "hooks/usageMetrics";
import { rootComponentPath } from "api/usage";
import {
  FunctionBreakdownMetricActionCompute,
  FunctionBreakdownMetricCalls,
  FunctionBreakdownMetricDatabaseBandwidth,
  FunctionBreakdownMetricVectorBandwidth,
  TeamUsageByFunctionChart,
} from "./TeamUsageByFunctionChart";

const meta: Meta<typeof TeamUsageByFunctionChart> = {
  component: TeamUsageByFunctionChart,
};

export default meta;

const team = {
  id: 42,
  name: "My amazing team",
  creator: 1,
  slug: "my-amazing-team",
  suspended: false,
  referralCode: "MYAMAZ1341",
  referredBy: null,
};

const project: ProjectDetails = {
  id: 42,
  teamId: 42,
  slug: "my-project",
  name: "My Project",
  isDemo: false,
  createTime: 0,
};

const deployments: DeploymentResponse[] = [
  {
    kind: "cloud",
    id: 10,
    projectId: 42,
    name: "fabulous-goldfish-42",
    deploymentType: "dev",
    createTime: 0,
    creator: 1,
    previewIdentifier: null,
  },
  {
    kind: "cloud",
    id: 20,
    projectId: 42,
    name: "friendly-dog-64",
    deploymentType: "dev",
    createTime: 0,
    creator: 2,
    previewIdentifier: null,
  },
  {
    kind: "cloud",
    id: 30,
    projectId: 42,
    name: "wandering-fish-513",
    deploymentType: "prod",
    createTime: 0,
    creator: 1,
    previewIdentifier: null,
  },
];

const rows: AggregatedFunctionMetrics[] = [
  {
    projectId: 42,
    deploymentId: 30,
    function: "module.js:getLeague",
    callCount: 1,
    databaseIngressSize: 349567,
    databaseEgressSize: 12345,
    vectorIngressSize: 0,
    vectorEgressSize: 0,
    actionComputeTime: 0,
    componentPath: rootComponentPath,
  },
  {
    projectId: 42,
    deploymentId: 30,
    function: "folder/module.js:setProfile",
    callCount: 10_000,
    databaseIngressSize: 67,
    databaseEgressSize: 12345,
    vectorIngressSize: 0,
    vectorEgressSize: 0,
    actionComputeTime: 0,
    componentPath: rootComponentPath,
  },
  {
    projectId: 42,
    deploymentId: 30,
    function: "module.js:thisFunctionHasAReallyLongName",
    callCount: 2_000,
    databaseIngressSize: 34567,
    databaseEgressSize: 12345,
    vectorIngressSize: 0,
    vectorEgressSize: 0,
    actionComputeTime: 0,
    componentPath: rootComponentPath,
  },
  {
    projectId: 42,
    deploymentId: 30,
    function: "module.js:setAvatar",
    callCount: 9_000,
    databaseIngressSize: 34567,
    databaseEgressSize: 12345,
    vectorIngressSize: 0,
    vectorEgressSize: 0,
    actionComputeTime: 0,
    componentPath: rootComponentPath,
  },
  {
    projectId: 42,
    deploymentId: 20,
    function: "module.js:getBalls",
    callCount: 1_000,
    databaseIngressSize: 34567,
    databaseEgressSize: 12345,
    vectorIngressSize: 0,
    vectorEgressSize: 0,
    actionComputeTime: 0,
    componentPath: rootComponentPath,
  },
  {
    projectId: 42,
    deploymentId: 20,
    function: "/api/endpoint",
    callCount: 12345,
    databaseIngressSize: 34567,
    databaseEgressSize: 12345,
    vectorIngressSize: 0,
    vectorEgressSize: 0,
    actionComputeTime: 0,
    componentPath: rootComponentPath,
  },
  {
    projectId: 42,
    deploymentId: 10,
    function: "devMerge.js:example",
    callCount: 40,
    databaseIngressSize: 34567,
    databaseEgressSize: 12345,
    vectorIngressSize: 0,
    vectorEgressSize: 0,
    actionComputeTime: 0,
    componentPath: rootComponentPath,
  },
  {
    projectId: 42,
    deploymentId: 30,
    function: "folder/module.js:setScore",
    callCount: 16_000,
    databaseIngressSize: 34567,
    databaseEgressSize: 12345,
    vectorIngressSize: 0,
    vectorEgressSize: 0,
    actionComputeTime: 0,
    componentPath: rootComponentPath,
  },
  {
    projectId: 42,
    deploymentId: 30,
    function: "module.js:default",
    callCount: 21_034,
    databaseIngressSize: 34567,
    databaseEgressSize: 12345,
    vectorIngressSize: 10,
    vectorEgressSize: 20,
    actionComputeTime: 0,
    componentPath: rootComponentPath,
  },
  {
    projectId: 42,
    deploymentId: 20,
    function: "devMerge.js:example",
    callCount: 2,
    databaseIngressSize: 34567,
    databaseEgressSize: 12345,
    vectorIngressSize: 50,
    vectorEgressSize: 10,
    actionComputeTime: 0,
    componentPath: rootComponentPath,
  },
  {
    projectId: 2,
    deploymentId: 31,
    function: "former_project_function.js:default",
    callCount: 22_022,
    databaseIngressSize: 34567,
    databaseEgressSize: 12345,
    vectorIngressSize: 0,
    vectorEgressSize: 0,
    actionComputeTime: 0,
    componentPath: rootComponentPath,
  },
  {
    projectId: 42,
    deploymentId: 30,
    function: "module.js:getBalls",
    callCount: 25_431,
    databaseIngressSize: 34567,
    databaseEgressSize: 12345,
    vectorIngressSize: 0,
    vectorEgressSize: 0,
    actionComputeTime: 0,
    componentPath: rootComponentPath,
  },
  {
    projectId: 42,
    deploymentId: 30,
    function: "module.js:getBall",
    callCount: 30_000,
    databaseIngressSize: 34567,
    databaseEgressSize: 12345,
    vectorIngressSize: 0,
    vectorEgressSize: 0,
    actionComputeTime: 0.0023,
    componentPath: rootComponentPath,
  },
  {
    projectId: 42,
    deploymentId: 30,
    function: "folder/module.js:getScore",
    callCount: 20_000,
    databaseIngressSize: 34567,
    databaseEgressSize: 12345,
    vectorIngressSize: 0,
    vectorEgressSize: 0,
    actionComputeTime: 0.123,
    componentPath: rootComponentPath,
  },
  {
    projectId: 42,
    deploymentId: 20,
    function: "folder/module.js:getScore",
    callCount: 200,
    databaseIngressSize: 34567,
    databaseEgressSize: 12345,
    vectorIngressSize: 0,
    vectorEgressSize: 0,
    actionComputeTime: 32.4456778,
    componentPath: rootComponentPath,
  },
];

export const Default: StoryObj<typeof TeamUsageByFunctionChart> = {
  args: {
    rows,
    team,
    project,
    deployments,
    maxValue: Math.max(...rows.map(FunctionBreakdownMetricCalls.getTotal)),
    metric: FunctionBreakdownMetricCalls,
  },
};

export const ForDeletedProject: StoryObj<typeof TeamUsageByFunctionChart> = {
  args: {
    rows,
    team,
    project: null,
    deployments: [],
    maxValue: Math.max(...rows.map(FunctionBreakdownMetricCalls.getTotal)),
    metric: FunctionBreakdownMetricCalls,
  },
};

export const DatabaseBandwidth: StoryObj<typeof TeamUsageByFunctionChart> = {
  args: {
    rows,
    team,
    project,
    deployments,
    maxValue: Math.max(
      ...rows.map(FunctionBreakdownMetricDatabaseBandwidth.getTotal),
    ),
    metric: FunctionBreakdownMetricDatabaseBandwidth,
  },
};

export const ActionCompute: StoryObj<typeof TeamUsageByFunctionChart> = {
  args: {
    rows,
    team,
    project,
    deployments,
    maxValue: Math.max(
      ...rows.map(FunctionBreakdownMetricActionCompute.getTotal),
    ),
    metric: FunctionBreakdownMetricActionCompute,
  },
};

export const VectorBandwidth: StoryObj<typeof TeamUsageByFunctionChart> = {
  args: {
    rows,
    team,
    project,
    deployments,
    maxValue: Math.max(
      ...rows.map(FunctionBreakdownMetricVectorBandwidth.getTotal),
    ),
    metric: FunctionBreakdownMetricVectorBandwidth,
  },
};
