import fs from "fs";
import path from "path";

import react from "@vitejs/plugin-react";

export default {
  plugins: [
    react(),
    {
      buildEnd() {
        const deps = [];
        for (const id of this.getModuleIds()) {
          const m = this.getModuleInfo(id);
          if (m !== null && !m.isExternal) {
            for (const target of m.importedIds) {
              deps.push({ source: m.id, target });
            }
          }
        }

        fs.writeFileSync(
          path.join(__dirname, "parcel-bundle-buddy.json"),
          JSON.stringify(deps, null, 2),
        );
      },
    },
  ],
  build: {
    sourcemap: true,
  },
};
