const PROJECT_COLORS = [
  '#3D7AED', // blue
  '#2DD4A0', // green
  '#F0993E', // orange
  '#A78BFA', // purple
  '#EF5757', // red
  '#FACC15', // yellow
  '#06B6D4', // cyan
  '#EC4899', // pink
];

export function getProjectColor(projectKey: string): string {
  let hash = 0;
  for (let i = 0; i < projectKey.length; i++) {
    hash = ((hash << 5) - hash + projectKey.charCodeAt(i)) | 0;
  }
  return PROJECT_COLORS[Math.abs(hash) % PROJECT_COLORS.length];
}
