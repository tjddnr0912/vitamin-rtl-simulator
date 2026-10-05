module top;
  logic clk = 0; logic [3:0] v; int mc, mf, mt; logic [7:0] r;
  class C;
    function int meth(logic [3:0] x);
      case (x) inside 4'b1?00: return 1; [4'd1:4'd3]: return 2; default: return 0; endcase
    endfunction
  endclass
  function automatic int fn(input logic [3:0] x);
    case (x) inside 4'b1?00: fn = 1; [4'd1:4'd3]: fn = 2; default: fn = 0; endcase
  endfunction
  task automatic tk(input logic [3:0] x, output int o);
    case (x) inside 4'b1?00: o = 1; [4'd1:4'd3]: o = 2; default: o = 0; endcase
  endtask
  always_comb begin
    case (v) inside 4'b1?00: mc = 1; [4'd1:4'd3]: mc = 2; default: mc = 0; endcase
  end
  always_ff @(posedge clk) begin
    case (v) inside 4'b1?00: r <= 8'hAB; [4'd1:4'd3]: r <= 4'h5; default: r <= 16'hFFEE; endcase
  end
  logic [3:0] vals [0:4] = '{4'b1000, 4'b0010, 4'b0110, 4'b1100, 4'b0011};
  initial begin
    C c; c = new;
    for (int i = 0; i < 5; i++) begin
      v = vals[i]; #1 clk = 1; #1 clk = 0;
      tk(v, mt); mf = fn(v);
      // nested case inside
      case (v) inside
        [4'd0:4'd7]: case (v) inside 4'b0?10: $display("nest A"); default: $display("nest B"); endcase
        default: $display("nest C");
      endcase
      $display("v=%b comb=%0d ff=%h fn=%0d task=%0d meth=%0d", v, mc, r, mf, mt, c.meth(v));
    end
    #10 $finish;
  end
endmodule
