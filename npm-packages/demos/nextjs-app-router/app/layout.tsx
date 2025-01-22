import "./globals.css";
import type { Metadata } from "next";
import { Inter } from "next/font/google";
import { ConvexClientProvider } from "./ConvexClientProvider";
import { cn } from "@/lib/utils";

const inter = Inter({ subsets: ["latin"] });

export const metadata: Metadata = {
  title: "My App Title",
  description: "My app description",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body className={cn(inter.className, "bg-orange-200")}>
        <ConvexClientProvider>{children}</ConvexClientProvider>
      </body>
    </html>
  );
}
