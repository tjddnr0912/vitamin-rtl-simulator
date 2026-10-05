module leaf(input logic [1:0] s, output logic [1:0] y);
  always_latch begin
    unique case (s) 2'd1: y = 1; 2'd2: y = 2; endcase
  end
endmodule
module top;
  logic [1:0] s, y;
  leaf u(.s(s), .y(y));
  initial begin
    s = 2'd1;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 s = 2'd0;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
