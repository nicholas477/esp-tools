using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Globalization;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Data;
using System.Windows.Documents;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Navigation;
using System.Windows.Shapes;
using Newtonsoft.Json;

public class Asset
{
    public uint Index { get; set; }
    public string Name { get; set; }
    public bool Export { get; set; }
    public List<uint> Children { get; set; }
}

public class FileAssets
{
    public uint RootIndex { get; set; }
    public List<Asset> Assets { get; set; }
}

namespace esp_tools_gui
{
    public class IntToVisibilityConverter : IValueConverter
    {
        public object Convert(object value, Type targetType, object parameter, CultureInfo culture)
        {
            if (value is int count && count > 0)
                return Visibility.Visible;

            return Visibility.Collapsed;
        }

        public object ConvertBack(object value, Type targetType, object parameter, CultureInfo culture) => throw new NotImplementedException();
    }

    /// <summary>
    /// Interaction logic for MainWindow.xaml
    /// </summary>
    public partial class MainWindow : Window
    {
        public ObservableCollection<ReferenceTreeItem> TreeRootNodes { get; set; }
            = new ObservableCollection<ReferenceTreeItem>();

        public MainWindow()
        {
            InitializeComponent();
            this.DataContext = this;

            StartLoad();
        }

        private async void StartLoad()
        {
            LoadingPopup.IsOpen = true;
            try
            {
                ReferenceTreeItem node = await Task.Run(() => LoadSampleData());
                if (node != null)
                {
                    TreeRootNodes.Clear();
                    TreeRootNodes.Add(node);
                }
            }
            finally
            {
                LoadingPopup.IsOpen = false;
            }
        }

        private ReferenceTreeItem LoadSampleData()
        {
            string json = EspTools.ScanToJson("I:\\SteamLibrary\\steamapps\\common\\Morrowind\\Data Files\\tr_mw_flora_tree_indoril_elm.ESP");
            Console.WriteLine(json);

            FileAssets assets = JsonConvert.DeserializeObject<FileAssets>(json);
            Asset asset = assets.Assets.FirstOrDefault(a => a.Index == assets.RootIndex);
            if (asset != null)
            {
                ReferenceTreeItem node = new ReferenceTreeItem
                {
                    Title = asset.Name,
                    ShouldExport = asset.Export,
                    Id = asset.Index
                };
                BuildReferenceTree(node, assets);

                Console.WriteLine($"Root Node: {node.Title}, ShouldExport: {node.ShouldExport}, Id: {node.Id}");

                return node;
            }
            return null;
        }

        // Recursively build the tree structure from the assets
        private void BuildReferenceTree(ReferenceTreeItem currentNode, FileAssets assets)
        {
            if (currentNode == null || assets == null)
                return;

            Asset currentAsset = assets.Assets.FirstOrDefault(a => a.Index == currentNode.Id);
            if (currentAsset != null && currentAsset.Children != null)
            {
                foreach (uint childIndex in currentAsset.Children)
                {
                    Asset childAsset = assets.Assets.FirstOrDefault(a => a.Index == childIndex);
                    if (childAsset != null)
                    {
                        ReferenceTreeItem childNode = new ReferenceTreeItem
                        {
                            Title = childAsset.Name,
                            ShouldExport = childAsset.Export,
                            Id = childAsset.Index
                        };
                        currentNode.Children.Add(childNode);
                        BuildReferenceTree(childNode, assets);
                    }
                }
            }
        }
    }
}
