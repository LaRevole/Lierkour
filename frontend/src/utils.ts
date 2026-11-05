export const isSerialCategory = (category?: string | null): boolean => {
  if (!category) return false;
  const cat = category.toLowerCase().trim();
  return (
    cat.includes('сериал') ||
    cat.includes('аниме') ||
    cat.includes('series') ||
    cat.includes('anime') ||
    cat.includes('дорам')
  );
};

