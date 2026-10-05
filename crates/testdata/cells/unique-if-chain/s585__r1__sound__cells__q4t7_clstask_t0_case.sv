class K;
  int r;
  task t(input logic x, input logic z);
    r = 0; unique case (1'b1) x: r = 1; z: r = 2; endcase
  endtask
endclass
module dut(input logic a, input logic b);
  K k;
  initial begin k = new; k.t(a, b); fork k.t(a, b); join_none end
endmodule
module top;
  logic a, b;
  dut u(.a(a), .b(b));
  initial begin a = 0; b = 1; #1 b = 0; #1 $finish; end
endmodule
