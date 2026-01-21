using System.ComponentModel;

namespace SpaceThumbnails.ControlPanel
{
    public class FormatItem : INotifyPropertyChanged
    {
        public string Extension { get; set; }
        public string Guid { get; set; }
        public string Description { get; set; } // Add this line

        private bool _isEnabled;
        public bool IsEnabled
        {
            get => _isEnabled;
            set
            {
                if (_isEnabled != value)
                {
                    _isEnabled = value;
                    PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(IsEnabled)));
                }
            }
        }

        public event PropertyChangedEventHandler PropertyChanged;
    }
}
