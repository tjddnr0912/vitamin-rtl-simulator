module top;
  function automatic logic [3:0] fx(input integer a); if (a > 5) fx = 4'd1; endfunction
  localparam logic [3:0] Q = fx(2);
  if (1) begin : gb
    case (4'd0)
      K: begin : gk wire [7:0] w = 8'd200; initial #1 $display("@k %0d bits=%0d", w, $bits(w)); end
      default: begin : gd wire [3:0] w = 4'd9; initial #1 $display("@def %0d bits=%0d", w, $bits(w)); end
    endcase
    localparam logic [3:0] K = Q;
  end
  initial #5 $finish;
endmodule
