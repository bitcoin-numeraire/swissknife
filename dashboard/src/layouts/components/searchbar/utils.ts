import type { NavSectionProps } from 'src/components/nav-section';

import { hasAllPermissions } from 'src/auth/permissions';

// ----------------------------------------------------------------------

type NavItem = {
  title: string;
  path: string;
  children?: NavItem[];
  permissions?: string[];
};

type OutputItem = {
  title: string;
  path: string;
  group: string;
};

function filterByPermissions(items: NavItem[], userPermissions: string[]): NavItem[] {
  return items
    .filter((item) => !item.permissions || hasAllPermissions(item.permissions, userPermissions))
    .map((item) => ({
      ...item,
      children: item.children ? filterByPermissions(item.children, userPermissions) : undefined,
    }));
}

const flattenNavItems = (navItems: NavItem[], parentGroup?: string): OutputItem[] => {
  let flattenedItems: OutputItem[] = [];

  navItems.forEach((navItem) => {
    const currentGroup = parentGroup ? `${parentGroup}-${navItem.title}` : navItem.title;
    const groupArray = currentGroup.split('-');

    flattenedItems.push({
      title: navItem.title,
      path: navItem.path,
      group: groupArray.length > 2 ? `${groupArray[0]}.${groupArray[1]}` : groupArray[0],
    });

    if (navItem.children) {
      flattenedItems = flattenedItems.concat(flattenNavItems(navItem.children, currentGroup));
    }
  });
  return flattenedItems;
};

export function flattenNavSections(
  navSections: NavSectionProps['data'],
  userPermissions: string[] = []
): OutputItem[] {
  const filteredSections = navSections.map((navSection) => ({
    ...navSection,
    items: filterByPermissions(navSection.items as NavItem[], userPermissions),
  }));

  return filteredSections.flatMap((navSection) =>
    flattenNavItems(navSection.items as NavItem[], navSection.subheader)
  );
}

// ----------------------------------------------------------------------

type ApplyFilterProps = {
  query: string;
  inputData: OutputItem[];
};

export function applyFilter({ inputData, query }: ApplyFilterProps): OutputItem[] {
  if (!query) return inputData;

  return inputData.filter(({ title, path, group }) =>
    [title, path, group].some((field) => field?.toLowerCase().includes(query.toLowerCase()))
  );
}
