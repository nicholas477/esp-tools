using System;
using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Runtime.CompilerServices;

namespace esp_tools_gui
{
    public class ReferenceTreeItem : INotifyPropertyChanged
    {
        private string _title;
        private bool _shouldExport;

        private uint _id;
        public string Title
        {
            get => _title;
            set { _title = value; OnPropertyChanged(); }
        }

        public bool ShouldExport
        {
            get => _shouldExport;
            set { _shouldExport = value; OnPropertyChanged(); }
        }

        public uint Id
        {
            get => _id;
            set { _id = value; OnPropertyChanged(); }
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
