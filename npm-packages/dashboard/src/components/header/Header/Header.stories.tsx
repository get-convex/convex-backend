import { Meta, StoryObj } from "@storybook/react";
import { UserProfile, UserProvider } from "@auth0/nextjs-auth0/client";
import { BreadcrumbLink } from "../BreadcrumbLink/BreadcrumbLink";
import { Breadcrumbs } from "../Breadcrumbs/Breadcrumbs";
import { NavBar } from "../NavBar/NavBar";
import { Header } from "./Header";

const mockUser = {} as unknown as UserProfile;

export default {
  component: Header,
  render: (args) => (
    <UserProvider user={mockUser}>
      <Header {...args} />
    </UserProvider>
  ),
} as Meta<typeof Header>;

export const Default: StoryObj<typeof Header> = {
  args: {
    user: {
      name: "Test User",
      href: "/user",
    },
  },
};

export const WithLinks: StoryObj<typeof Header> = {
  args: {
    user: {
      name: "Test User",
    },
    children: (
      <NavBar
        activeLabel="One"
        items={[
          {
            label: "One",
            href: "/one",
          },
          {
            label: "Two",
            href: "/two",
          },
          {
            label: "Three",
            href: "/three",
          },
        ]}
      />
    ),
  },
};

export const WithBreadcrumbComponents: StoryObj<typeof Header> = {
  args: {
    user: {
      name: "Test User",
    },
    children: (
      <Breadcrumbs>
        <BreadcrumbLink href="/projects">Projects</BreadcrumbLink>
        <BreadcrumbLink href="/projects/abc">My Project</BreadcrumbLink>
      </Breadcrumbs>
    ),
  },
};
