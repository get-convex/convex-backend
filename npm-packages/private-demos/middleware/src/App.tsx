import { api } from "../convex/_generated/api";
import {
  justSessionQueries,
  useSessionQuery,
  useSessionQueryOverload,
} from "./hooks/useServerSession";

const sqApi = justSessionQueries(api);

export default function App() {
  const a = useSessionQuery(sqApi.sessions.unvalidatedQueryNoArgNoObject);
  // without the overload, no type error
  const b = useSessionQuery(sqApi.sessions.unvalidatedQueryWithArgNoObject);
  const c = useSessionQuery(sqApi.sessions.unvalidatedQueryWithArgNoObject, {
    a: 1,
  });

  // with an overload it works
  const d = useSessionQueryOverload(
    sqApi.sessions.unvalidatedQueryNoArgNoObject,
  );
  const e = useSessionQueryOverload(
    // @ts-expect-error - with the overload this correctly errors
    sqApi.sessions.unvalidatedQueryWithArgNoObject,
  );
  const f = useSessionQueryOverload(
    sqApi.sessions.unvalidatedQueryWithArgNoObject,
    { a: 1 },
  );
  console.log(a, b, c, d, e, f);
  return (
    <main>
      <div>Hello, world!</div>
    </main>
  );
}
