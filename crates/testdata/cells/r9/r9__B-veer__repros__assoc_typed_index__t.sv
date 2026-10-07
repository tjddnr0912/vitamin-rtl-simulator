module top;
  bit [7:0] mem [bit [31:0]];    // associative array indexed by a packed type (IEEE 1800-2017 7.8.4)
  bit [7:0] m2  [int unsigned];
  initial begin
    mem[32'hF000_0000] = 8'h5A;
    m2[32'hF000_0001]  = 8'hA5;
    $display("mem=%h m2=%h n=%0d", mem[32'hF000_0000], m2[32'hF000_0001], mem.num());
    $finish;
  end
endmodule
