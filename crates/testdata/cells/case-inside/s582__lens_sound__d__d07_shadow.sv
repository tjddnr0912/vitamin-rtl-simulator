module sub #(parameter int LO = 1, parameter int HI = 3) (input bit [3:0] x, output int m);
  always_comb begin
    case (x) inside [LO:HI]: m = 1; default: m = 0; endcase
  end
endmodule
module top;
  bit [3:0] lo = 4'd9, hi = 4'd9, x, v = 4'd9;
  int m1, m2, m3, m4, ms1, ms2;
  function automatic int ff(input bit [3:0] a);
    bit [3:0] lo;
    lo = 4'd2;
    case (a) inside [lo:4'd5]: return 1; default: return 0; endcase
  endfunction
  sub #(.LO(4), .HI(6)) u1 (.x(x), .m(ms1));
  sub #(.LO(8), .HI(9)) u2 (.x(x), .m(ms2));
  initial begin
    x = 4'd4;
    begin : blk
      bit [3:0] lo; bit [3:0] v;
      lo = 4'd3; v = 4'd4;
      case (x) inside [lo:4'd5]: m1 = 1; default: m1 = 0; endcase
      case (x) inside v: m2 = 1; default: m2 = 0; endcase
      case (v) inside [4'd4:4'd4]: m3 = 1; default: m3 = 0; endcase
    end
    m4 = ff(4'd3);
    #1 $display("m1=%0d m2=%0d m3=%0d m4=%0d ms1=%0d ms2=%0d lo=%0d v=%0d", m1, m2, m3, m4, ms1, ms2, lo, v);
    #1 $finish;
  end
endmodule
