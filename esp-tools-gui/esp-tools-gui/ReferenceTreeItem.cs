using System;
using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using System.Windows;

namespace esp_tools_gui
{
    public class ReferenceTreeItem : INotifyPropertyChanged
    {
        private Asset _asset = null;

        public string Title
        {
            get => _asset?.Name ?? string.Empty;
        }

        public Asset Asset
        {
            get => _asset;
            set
            {
                if (_asset != value)
                {
                    _asset = value;
                    OnPropertyChanged();

                    if (_asset != null)
                    {
                        // Don't expand master assets at first, but expand all other assets by default
                        IsExpanded = !_asset.IsMasterAsset;

                        OnPropertyChanged(nameof(Title));
                        OnPropertyChanged(nameof(ShouldExport));
                        OnPropertyChanged(nameof(IsMasterAsset));
                        OnPropertyChanged(nameof(GetCheckboxVisibility));
                        OnPropertyChanged(nameof(Id));

                        _asset.PropertyChanged += (sender, e) =>
                        {
                            if (e.PropertyName == nameof(Asset.Export))
                            {
                                OnPropertyChanged(nameof(ShouldExport));
                            }
                        };
                    }
                }
            }
        }

        public bool ShouldExport
        {
            get => _asset?.Export ?? false;
            set
            {
                if (_asset != null)
                {
                    _asset.Export = value;
                    OnPropertyChanged();
                }
            }
        }

        public bool IsMasterAsset
        {
            get => _asset?.IsMasterAsset ?? false;
        }

        public Visibility GetCheckboxVisibility
        {
            get => IsMasterAsset ? Visibility.Collapsed : Visibility.Visible;
        }

        private bool _isExpanded;

        public bool IsExpanded
        {
            get => _isExpanded;
            set
            {
                if (_isExpanded != value)
                {
                    _isExpanded = value;
                    OnPropertyChanged();
                }
            }
        }

        public uint Id
        {
            get => _asset?.Index ?? 0;
        }

        // Children collection must also use the new class name
        public ObservableCollection<ReferenceTreeItem> Children { get; set; } = new ObservableCollection<ReferenceTreeItem>();

        public event PropertyChangedEventHandler PropertyChanged;
        protected void OnPropertyChanged([CallerMemberName] string propertyName = null)
        {
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
        }
    }
}
