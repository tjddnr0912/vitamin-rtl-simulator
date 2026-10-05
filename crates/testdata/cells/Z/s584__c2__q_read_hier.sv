module top;
  logic [1:0] s, y;
  leafq u(.s(s), .y(y));
  initial begin
    s = 2'd1;
    $display("r0 y=%b", y);
    #0 $display("r1 y=%b", y);
    #0 $display("r2 y=%b", y);
    #0 $display("r3 y=%b", y);
  end
  initial #1 $finish;
endmodule
module leafq(input logic [1:0] s, output logic [1:0] y);
  always_comb begin
    y = 0;
    unique case (s) 2'd1: y = 1; 2'd2: y = 2; endcase
  end
endmodule
