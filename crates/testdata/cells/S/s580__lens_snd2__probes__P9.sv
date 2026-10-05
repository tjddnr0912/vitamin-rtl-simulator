`timescale 1ns/1ns
module t;
  string s = "a";
  logic [7:0] ua [0:1];
  logic [7:0] q [$];
  initial begin
    ua[0] = 8'h61; ua[1] = 8'h61; q.push_back(8'h61);
    #1;
`ifdef STRV
    $display("STRV %b %b", s ==? 8'b0110_000x, s ==? 8'b0110_001x);
`elsif SFMT
    $display("SFMT %b", $sformatf("%s", s) ==? 8'b0110_000x);
`elsif UARR
    $display("UARR %b", ua ==? 8'b0110_000x);
`elsif QUEUE
    $display("QUEUE %b", q ==? 8'b0110_000x);
`elsif STRI
    $display("STRI %b", s inside {8'b0110_000x});
`endif
    #1 $finish;
  end
endmodule
