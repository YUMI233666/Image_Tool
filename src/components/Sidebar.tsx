export type NavTab = "input" | "function" | "params" | "run" | "results";

interface NavItem { id: NavTab; icon: string; label: string }
const NAV: NavItem[] = [
  { id:"input",    icon:"📥", label:"输入" },
  { id:"function", icon:"🔧", label:"功能" },
  { id:"params",   icon:"⚙️", label:"参数" },
  { id:"run",      icon:"▶",  label:"运行" },
  { id:"results",  icon:"📊", label:"结果" },
];

interface Props {
  active: NavTab;
  onNav: (t: NavTab) => void;
  isRunning: boolean;
  failedCount: number;
  onStart: () => void;
  onCancel: () => void;
  theme: "dark" | "light";
  onThemeToggle: () => void;
}

export default function Sidebar({ active, onNav, isRunning, failedCount, onStart, onCancel, theme, onThemeToggle }: Props) {
  return (
    <aside className="sidebar">
      <div className="sidebar-logo">
        <span className="sidebar-logo-icon">🎨</span>
        <div>
          <div className="sidebar-logo-text">Art Tool</div>
          <div className="sidebar-logo-version">v1.3.0</div>
        </div>
      </div>

      <nav className="sidebar-nav">
        {NAV.map(item => (
          <button
            key={item.id}
            type="button"
            className={"sidebar-nav-item" + (active === item.id ? " active" : "")}
            onClick={() => onNav(item.id)}
          >
            <span className="sidebar-nav-icon">{item.icon}</span>
            <span>{item.label}</span>
            {item.id === "results" && failedCount > 0 && (
              <span className="sidebar-nav-badge">{failedCount}</span>
            )}
          </button>
        ))}
      </nav>

      <button
        type="button"
        className={"sidebar-start-btn" + (isRunning ? " running" : "")}
        onClick={isRunning ? onCancel : onStart}
      >
        {isRunning ? "⏹ 取消" : "▶ 开始处理"}
      </button>

      <div className="sidebar-footer">
        <span className="sidebar-footer-version">MIT License</span>
        <button type="button" className="theme-toggle" onClick={onThemeToggle} title="切换主题">
          {theme === "dark" ? "☀" : "🌙"}
        </button>
      </div>
    </aside>
  );
}
