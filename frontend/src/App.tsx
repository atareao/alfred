import { BrowserRouter, Routes, Route } from "react-router-dom";
import { ConfigProvider } from "antd";
import { alfredTheme } from "./theme";
import { AppLayout } from "./components/AppLayout";

function App() {
  return (
    <BrowserRouter>
      <ConfigProvider theme={alfredTheme}>
        <Routes>
          <Route path="*" element={<AppLayout />} />
        </Routes>
      </ConfigProvider>
    </BrowserRouter>
  );
}

export default App;