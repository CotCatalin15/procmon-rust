using System;
using System.Windows;
using System.Windows.Controls;
using uniffi.procmon;
using static System.Formats.Asn1.AsnWriter;

namespace ProcmonUI
{
    public partial class MainWindow : Window
    {
        private readonly EventViewModel _viewModel;

        private ProcmonCore _core;

        public MainWindow()
        {
            InitializeComponent();

            _viewModel = new EventViewModel(@"Data Source=D:\\Procmondb\events.db;Version=3;");
            DataContext = _viewModel;

            // Start background updates
            StartBackgroundUpdates();
        }

        private async void ScrollViewer_ScrollChanged(object sender, ScrollChangedEventArgs e)
        {
            var scrollViewer = (ScrollViewer)sender;
            if (scrollViewer.VerticalOffset >= scrollViewer.ScrollableHeight - 100)
            {
                await _viewModel.LoadMoreEventsAsync();
            }
        }

        private async void StartBackgroundUpdates()
        {
            while (true)
            {
                await Task.Delay(5000); // Check for updates every 5 seconds
                await _viewModel.LoadMoreEventsAsync();
            }
        }
    }
}