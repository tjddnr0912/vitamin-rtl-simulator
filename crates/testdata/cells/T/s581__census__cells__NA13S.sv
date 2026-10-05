package pk; localparam logic signed [3:0] KS = -1; endpackage
module top;
  localparam logic [3:0] KS = 4'd5;
  case (pk::KS)
    4'b1111: begin : g_hit wire [7:0] w = 8'd1; initial #1 $display("NA13S hit %0d", w); end
    4'd5: begin : g_five wire [7:0] w = 8'd2; initial #1 $display("NA13S five %0d", w); end
    default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("NA13S def %0d", w); end
  endcase
endmodule
