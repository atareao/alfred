import { BrowserRouter, Routes, Route } from "react-router-dom";
import { ConfigProvider } from "antd";
import { valetTheme } from "./theme";
import { AppLayout } from "./components/AppLayout";

function App() {
  return (
    <BrowserRouter>
      <ConfigProvider theme={valetTheme}>
        <Routes>
          <Route path="*" element={<AppLayout />} />
        </Routes>
      </ConfigProvider>
    </BrowserRouter>
  );
}

export default App;