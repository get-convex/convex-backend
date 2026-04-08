import { httpRouter } from "convex/server";
import { registerRoutes } from "../../../httpComponent/registerRoutes";

const http = httpRouter();
registerRoutes(http);
export default http;
