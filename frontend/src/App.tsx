import { BrowserRouter, Routes, Route } from "react-router-dom";
import { ConfigProvider } from "antd";
import { alfredTheme } from "./theme";
import { AppLayout } from "./components/AppLayout";
import { StatsDashboard } from "./pages/StatsDashboard";

function App() {
  return (
    <BrowserRouter>
      <ConfigProvider theme={alfredTheme}>
        <Routes>
          <Route path="/stats" element={<AppLayout><StatsDashboard /></AppLayout>} />
          <Route path="*" element={<AppLayout />} />
        </Routes>
      </ConfigProvider>
    </BrowserRouter>
  );
}

export default App;