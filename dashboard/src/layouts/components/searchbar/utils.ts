import type { NavSectionProps, NavItemBaseProps } from 'src/components/nav-section';

import { flattenArray } from 'src/utils/helper';

import { hasAllPermissions } from 'src/auth/permissions';

// ----------------------------------------------------------------------

type ItemProps = {
  group: string;
  title: string;
  path: string;
};

export function getAllItems({ data, permissions = [] }: { data: NavSectionProps['data']; permissions: string[] }) {
  const reduceItems = data.map((list) => handleLoop(list.items, list.subheader)).flat();

  const items = flattenArray(reduceItems)
    .filter((option) => !option.permissions || hasAllPermissions(option.permissions, permissions))
    .map((option) => {
      const group = splitPath(reduceItems, option.path);

      return {
        group: group && group.length > 1 ? group[0] : option.subheader,
        title: option.title,
        path: option.path,
      };
    });

  return items;
}

// ----------------------------------------------------------------------

type ApplyFilterProps = {
  inputData: ItemProps[];
  query: string;
};

export function applyFilter({ inputData, query }: ApplyFilterProps) {
  if (query) {
    inputData = inputData.filter(
      (item) => item.title.toLowerCase().indexOf(query.toLowerCase()) !== -1 || item.path.toLowerCase().indexOf(query.toLowerCase()) !== -1
    );
  }

  return inputData;
}

// ----------------------------------------------------------------------

export function splitPath(array: NavItemBaseProps[], key: string) {
  let stack = array.map((item) => ({ path: [item.title], currItem: item }));

  while (stack.length) {
    const { path, currItem } = stack.pop() as {
      path: string[];
      currItem: NavItemBaseProps;
    };

    if (currItem.path === key) {
      return path;
    }

    if (currItem.children?.length) {
      stack = stack.concat(
        currItem.children.map((item: NavItemBaseProps) => ({
          path: path.concat(item.title),
          currItem: item,
        }))
      );
    }
  }
  return null;
}

// ----------------------------------------------------------------------

export function handleLoop(array: any, subheader?: string) {
  return array?.map((list: any) => ({
    subheader,
    ...list,
    ...(list.children && { children: handleLoop(list.children, subheader) }),
  }));
}

// ----------------------------------------------------------------------

type GroupsProps = {
  [key: string]: ItemProps[];
};

export function groupItems(array: ItemProps[]) {
  const group = array.reduce((groups: GroupsProps, item) => {
    groups[item.group] = groups[item.group] || [];

    groups[item.group].push(item);

    return groups;
  }, {});

  return group;
}
