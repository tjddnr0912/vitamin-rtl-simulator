module top;
  function automatic int f(input logic [3:0] x);
    case (x) 4'd8: return 1; 4'd2: return 2; default: return 0; endcase
  endfunction
  localparam int P1 = f(4'b1000);
  localparam int P2 = f(4'd2);
  initial begin $display("P1=%0d P2=%0d", P1, P2); #10 $finish; end
endmodule
