import { BrowserRouter, Routes, Route } from "react-router-dom";
import { ConfigProvider, App as AntdApp } from "antd";
import { valetTheme } from "./theme";
import { AppLayout } from "./components/AppLayout";
import { ProfileProvider } from "./contexts/ProfileProvider";

function App() {
  return (
    <BrowserRouter>
      <ConfigProvider theme={valetTheme}>
        <AntdApp>
          <ProfileProvider>
            <Routes>
              <Route path="*" element={<AppLayout />} />
            </Routes>
          </ProfileProvider>
        </AntdApp>
      </ConfigProvider>
    </BrowserRouter>
  );
}

export default App;