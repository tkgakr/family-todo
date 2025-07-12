import { useState } from "react";

function App() {
  const [count, setCount] = useState(0);

  return (
    <div style={{ padding: "20px", fontFamily: "Arial, sans-serif" }}>
      <h1>家族用TODOアプリ</h1>
      <p>現在の開発状況: 基盤セットアップ中</p>
      
      <div style={{ marginTop: "20px" }}>
        <button
          type="button"
          onClick={() => setCount((count) => count + 1)}
          style={{
            padding: "10px 20px",
            fontSize: "16px",
            backgroundColor: "#007acc",
            color: "white",
            border: "none",
            borderRadius: "4px",
            cursor: "pointer",
          }}
        >
          カウント: {count}
        </button>
      </div>
      
      <div style={{ marginTop: "20px", fontSize: "14px", color: "#666" }}>
        <p>技術スタック:</p>
        <ul>
          <li>React + TypeScript</li>
          <li>Vite</li>
          <li>Biome (リント・フォーマット)</li>
          <li>AWS サーバーレス</li>
        </ul>
      </div>
    </div>
  );
}

export default App;