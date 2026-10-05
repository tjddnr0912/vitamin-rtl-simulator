module top;
  localparam logic [3:0] EM = 4'd5;
  if (1) begin : gb
    typedef enum logic signed [3:0] {EM = -1, EZ = 0} e_t;
    case (EM)
      4'b1111: begin : g_hit wire [7:0] w = 8'd1; initial #1 $display("NA11S hit %0d", w); end
      4'd5: begin : g_five wire [7:0] w = 8'd2; initial #1 $display("NA11S five %0d", w); end
      default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("NA11S def %0d", w); end
    endcase
  end
endmodule
