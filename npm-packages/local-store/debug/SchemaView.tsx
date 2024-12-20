// import { formatValidator } from "./validatorHelpers";
// import { Highlight, themes } from "prism-react-renderer";

// export function SchemaView({ schemaString }: { schemaString: string }) {
//   const schema = JSON.parse(schemaString);
//   const tables = schema.tables || [];

//   return (
//     <div className="space-y-6">
//       {tables.map((table: any) => {
//         // Create a complete type object
//         const formattedType = formatValidator(table.documentType);
//         return (
//           <div key={table.tableName} className="border rounded-lg p-4">
//             <h3 className="text-lg font-semibold text-gray-800 mb-2">
//               {table.tableName}
//             </h3>

//             {/* Document Type */}
//             <div className="pl-4 space-y-2 mb-4">
//               <div className="text-sm font-medium text-gray-700">Schema:</div>
//               <Highlight
//                 theme={themes.github}
//                 code={formattedType}
//                 language="typescript"
//               >
//                 {({
//                   className,
//                   style,
//                   tokens,
//                   getLineProps,
//                   getTokenProps,
//                 }) => (
//                   <pre
//                     className={`${className} text-sm p-2 rounded overflow-auto`}
//                     style={style}
//                   >
//                     {tokens.map((line, i) => (
//                       <div key={i} {...getLineProps({ line })}>
//                         {line.map((token, key) => (
//                           <span key={key} {...getTokenProps({ token })} />
//                         ))}
//                       </div>
//                     ))}
//                   </pre>
//                 )}
//               </Highlight>
//             </div>

//             {/* Indexes */}
//             <div className="pl-4 space-y-2">
//               <div className="text-sm font-medium text-gray-700">Indexes:</div>
//               {table.indexes.map((index: any) => (
//                 <div key={index.indexDescriptor} className="text-sm pl-2">
//                   <span className="font-medium text-gray-700">
//                     {index.indexDescriptor}
//                   </span>
//                   <span className="text-gray-500 ml-2">
//                     ({index.fields.join(", ")})
//                   </span>
//                 </div>
//               ))}
//             </div>
//           </div>
//         );
//       })}
//     </div>
//   );
// }
export default null;
