using System;
using System.Collections.Generic;
using System.Data.SQLite;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

namespace ProcmonUI
{
    public class DataService
    {
        private readonly string _connectionString;

        public DataService(string connectionString)
        {
            _connectionString = connectionString;
        }

        public async Task<List<Event>> GetEventsAsync(long lastId, int pageSize)
        {
            var events = new List<Event>();

            using (var connection = new SQLiteConnection(_connectionString))
            {
                await connection.OpenAsync();
                using (var command = new SQLiteCommand(
                    "SELECT * FROM events WHERE Id > @lastId ORDER BY Id LIMIT @pageSize", connection))
                {
                    command.Parameters.AddWithValue("@lastId", lastId);
                    command.Parameters.AddWithValue("@pageSize", pageSize);

                    using (var reader = await command.ExecuteReaderAsync())
                    {
                        while (await reader.ReadAsync())
                        {
                            var e = new Event
                            {
                                Id = reader.GetInt64(0),
                                Timestamp = reader.GetInt64(1),
                                Pid = reader.GetInt32(2),
                                Uid = reader.GetInt32(3),
                                Tid = reader.GetString(4),
                                Path = reader.IsDBNull(5) ? null : (byte[])reader.GetValue(5),
                                Operation = reader.GetString(6),
                                AdditionalData = reader.IsDBNull(7) ? null : reader.GetString(7)
                            };
                            events.Add(e);
                        }
                    }
                }
            }

            return events;
        }

        public async Task<int> GetTotalCountAsync()
        {
            using (var connection = new SQLiteConnection(_connectionString))
            {
                await connection.OpenAsync();
                using (var command = new SQLiteCommand("SELECT COUNT(*) FROM events", connection))
                {
                    var result = await command.ExecuteScalarAsync();
                    return Convert.ToInt32(result);
                }
            }
        }
    }
}
