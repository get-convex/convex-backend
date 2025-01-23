import { Id } from "../convex/_generated/dataModel";

export type Message = {
  _id: Id;
  _creationTime: number;
  body: string;
  author: string;
};
