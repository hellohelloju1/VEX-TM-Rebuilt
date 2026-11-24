using Microsoft.Web.WebView2.Core;
using Microsoft.Web.WebView2.WinForms;
using System.Text;

internal static class Program {
 [STAThread] static int Main(string[] args) {
  if(args.Length<1)return 2;
  Console.InputEncoding=Encoding.UTF8;Console.OutputEncoding=Encoding.UTF8;
  ApplicationConfiguration.Initialize();
  using var form=new ReportWindow(Path.GetFullPath(args[0]),args.Length>1&&args[1]=="landscape",Console.In.ReadToEnd());
  Application.Run(form);return form.Result;
 }
}
internal sealed class ReportWindow:Form {
 readonly WebView2 web=new();readonly string output,html;
 readonly bool landscape;readonly string work=Path.Combine(Path.GetTempPath(),"tmrs-pdf-"+Guid.NewGuid());
 public int Result{get;private set;}=1;
 public ReportWindow(string output,bool landscape,string html){this.output=output;this.landscape=landscape;this.html=html;Opacity=0;ShowInTaskbar=false;Size=new(1100,800);web.Dock=DockStyle.Fill;Controls.Add(web);}
 protected override async void OnShown(EventArgs e){base.OnShown(e);try {
  Directory.CreateDirectory(work);
  var environment=await CoreWebView2Environment.CreateAsync(null,Path.Combine(work,"webview"));
  await web.EnsureCoreWebView2Async(environment);
  web.CoreWebView2.Settings.AreDevToolsEnabled=false;
  web.CoreWebView2.Settings.IsStatusBarEnabled=false;
  var ready=new TaskCompletionSource<bool>();
  web.NavigationCompleted+=(_,ev)=>{if(ev.IsSuccess)ready.TrySetResult(true);else ready.TrySetException(new Exception("Cannot load report: "+ev.WebErrorStatus));};
  var file=Path.Combine(work,"report.html");File.WriteAllText(file,html,new UTF8Encoding(false));web.Source=new Uri(file);
  await ready.Task.WaitAsync(TimeSpan.FromSeconds(30));
  await web.ExecuteScriptAsync("window.__pdfFontsReady=false;document.fonts.ready.then(()=>window.__pdfFontsReady=true)");
  bool fonts=false;for(int i=0;i<100;i++){if(await web.ExecuteScriptAsync("window.__pdfFontsReady") == "true"){fonts=true;break;}await Task.Delay(100);}
  if(!fonts)throw new Exception("Report font did not finish loading");
  var settings=environment.CreatePrintSettings();settings.ShouldPrintBackgrounds=true;settings.ShouldPrintHeaderAndFooter=false;
  settings.Orientation=landscape?CoreWebView2PrintOrientation.Landscape:CoreWebView2PrintOrientation.Portrait;
  if(!await web.CoreWebView2.PrintToPdfAsync(output,settings))throw new Exception("PDF export failed");
  Result=0;
 }catch(Exception error){Console.Error.WriteLine(error.Message);}finally{web.Dispose();Close();try{Directory.Delete(work,true);}catch{}}}
}
