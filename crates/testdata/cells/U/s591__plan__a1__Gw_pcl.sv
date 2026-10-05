module top;
  localparam [64:0] i = 65'h1_0000_0000_0000_0009;
  for (genvar i = 0; i < 2; i++) begin : g
    logic [31:0] k = 1;
    initial #3 case (k) i: $display("pcl %m hit"); default: $display("pcl %m miss"); endcase
    case (1) i: begin : gm initial #3 $display("gcl %m hit"); end default: begin : gd initial #3 $display("gcl %m miss"); end endcase
  end
  initial #100 $finish;
endmodule
