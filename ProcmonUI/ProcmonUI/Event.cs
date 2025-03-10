using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

namespace ProcmonUI
{
    public class Event
    {
        public long Id { get; set; }
        public long Timestamp { get; set; }
        public int Pid { get; set; }
        public int Uid { get; set; }
        public string Tid { get; set; }
        public byte[] Path { get; set; }
        public string Operation { get; set; }
        public string AdditionalData { get; set; }
    }

}
