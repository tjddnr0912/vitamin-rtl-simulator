module top;
  function automatic int f(input logic [3:0] x);
    if (x inside {4'b1?00}) return 1; else if (x inside {[4'd1:4'd3]}) return 2; else return 0;
  endfunction
  localparam int P1 = f(4'b1000);
  localparam int P2 = f(4'd2);
  localparam int P3 = f(4'd6);
  logic [P1+P2:0] w;
  initial begin $display("P1=%0d P2=%0d P3=%0d bits=%0d rt=%0d", P1, P2, P3, $bits(w), f(4'b1100)); #10 $finish; end
endmodule
