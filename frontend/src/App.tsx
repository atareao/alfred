import { ConfigProvider } from "antd";
import { alfredTheme } from "./theme";
import { AppLayout } from "./components/AppLayout";

function App() {
  return (
    <ConfigProvider theme={alfredTheme}>
      <AppLayout />
    </ConfigProvider>
  );
}

export default App;
