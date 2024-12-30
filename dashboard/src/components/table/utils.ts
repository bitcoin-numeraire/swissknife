// ----------------------------------------------------------------------

export function emptyRows(page: number, rowsPerPage: number, arrayLength: number) {
  return page ? Math.max(0, (1 + page) * rowsPerPage - arrayLength) : 0;
}

// ----------------------------------------------------------------------

/**
 * @example
 * const data = {
 *   calories: 360,
 *   align: 'center',
 *   more: {
 *     protein: 42,
 *   },
 * };
 *
 * const ex1 = getNestedProperty(data, 'calories');
 * console.log('ex1', ex1); // output: 360
 *
 * const ex2 = getNestedProperty(data, 'align');
 * console.log('ex2', ex2); // output: center
 *
 * const ex3 = getNestedProperty(data, 'more.protein');
 * console.log('ex3', ex3); // output: 42
 */
function getNestedProperty<T>(obj: T, key: string): any {
  return key.split('.').reduce((acc: any, part: string) => acc && acc[part], obj);
}

function descendingComparator<T>(a: T, b: T, orderBy: keyof T) {
  const aValue = getNestedProperty(a, orderBy as string);
  const bValue = getNestedProperty(b, orderBy as string);

  if (bValue < aValue) {
    return -1;
  }

  if (bValue > aValue) {
    return 1;
  }

  return 0;
}

// ----------------------------------------------------------------------

export function getComparator<Key extends keyof any>(
  order: 'asc' | 'desc',
  orderBy: Key
): (a: { [key in Key]: number | string }, b: { [key in Key]: number | string }) => number {
  return order === 'desc'
    ? (a, b) => descendingComparator(a, b, orderBy)
    : (a, b) => -descendingComparator(a, b, orderBy);
}
