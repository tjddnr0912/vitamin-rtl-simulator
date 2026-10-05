module top;
  function automatic logic [3:0] fx(input int a); logic [3:0] t; fx = t; endfunction
  if (1) begin : gb
    localparam logic [7:0] S = 8'd99;
    case (S)
      8'd99: begin : g localparam logic [3:0] P = fx(2); wire [7:0] w = 8'd200; initial #1 $display("@k P=%b bits=%0d", P, $bits(w)); end
      default: begin : g wire [3:0] w = 4'd9; initial #1 $display("@def bits=%0d", $bits(w)); end
    endcase
  end
  initial #5 $finish;
endmodule
