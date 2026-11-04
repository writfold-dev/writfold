WITH RECURSIVE walk AS (
  SELECT dst AS slug, 1 AS depth FROM dep WHERE src='a'
  UNION ALL
  SELECT d.dst, w.depth+1 FROM walk w JOIN dep d ON d.src=w.slug WHERE w.depth<2
) SELECT slug FROM walk WHERE slug <> 'a' GROUP BY slug HAVING MIN(depth)=2 ORDER BY slug;
