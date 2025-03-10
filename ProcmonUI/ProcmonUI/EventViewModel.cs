using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Linq;
using System.Runtime.CompilerServices;
using System.Text;
using System.Threading.Tasks;

namespace ProcmonUI
{
    public class EventViewModel : INotifyPropertyChanged
    {
        private readonly DataService _dataService;
        private long _lastId = 0;
        private bool _isLoading;
        public ObservableCollection<Event> Events { get; } = new ObservableCollection<Event>();

        public EventViewModel(string connectionString)
        {
            _dataService = new DataService(connectionString);
            LoadMoreEventsAsync();
        }

        public async Task LoadMoreEventsAsync()
        {
            if (_isLoading) return;
            _isLoading = true;

            var newEvents = await _dataService.GetEventsAsync(_lastId, 50);
            foreach (var e in newEvents)
            {
                Events.Add(e);
                _lastId = e.Id;
            }

            _isLoading = false;
        }

        public event PropertyChangedEventHandler PropertyChanged;
        protected void OnPropertyChanged([CallerMemberName] string propertyName = null)
        {
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
        }
    }
}
