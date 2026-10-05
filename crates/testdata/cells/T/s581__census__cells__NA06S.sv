module top;
  localparam [64:0] i = 65'd9;
  for (genvar i = 0; i < 2; i++) begin : g
    case (i)
      65'd9: begin : g_nine wire [7:0] w = 8'd1; initial #1 $display("NA06S nine %0d", w); end
      65'd1: begin : g_one wire [7:0] w = 8'd2; initial #1 $display("NA06S one %0d", w); end
      default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("NA06S def %0d", w); end
    endcase
  end
endmodule
